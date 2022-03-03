#![allow(unused)]
#![allow(dead_code)]

use chrono::Duration;
use chrono::prelude::*;
use firefox_user_installer::Downloader;
use firefox_user_installer::Error;
use regex::Regex;
use std::collections::HashMap;
use std::os::unix::process::CommandExt;
use std::path::{ Path, PathBuf };
use std::process;
use std::thread;
use std::sync::{Arc, Mutex};
use std::rc::Rc;
use std::env;

use gtk::prelude::*;
use gtk::{
    Application,
    ApplicationWindow,
    Button,
    Grid,
    Label,
    ComboBox,
    ComboBoxText,
    Window,
    ProgressBar,
    MessageDialog,
    DialogFlags,
    MessageType,
    ButtonsType
};
use gtk::glib;
use gtk::glib::clone;

enum Progress {
    Status(String),
    Percent(i64,i64),
}

enum InstallEvent {
    Progress(Progress),
    Success,
    Error(Error),
}

fn get_datadir() -> PathBuf {
    let dirs = directories::ProjectDirs::from("", "", "firefox-user-installer").unwrap();
    let result = PathBuf::from(dirs.data_dir());
    if !result.exists() {
        std::fs::create_dir_all(&result)
            .unwrap_or_else(|e| panic!("Failed to create directory {}: {}", result.display(), e));
    }
    result
}

fn get_cachedir(datadir: &Path) -> PathBuf {
    let result: PathBuf = [datadir, Path::new("cache")].iter().collect();
    if !result.exists() {
        std::fs::create_dir_all(&result)
            .unwrap_or_else(|e| panic!("Failed to create directory {}: {}", result.display(), e));
    }
    result
}

fn install<F>(datadir: &Path, appdir: &Path, browser: &str, architecture: &str, lang: &str, observer: F)  -> Result<(),Error>
where
    F: Fn(Progress)
{
    let cachedir = get_cachedir(datadir);
    let mut downloader = Downloader::new(&cachedir);

    let url = format!("https://download.mozilla.org/?product={}&os={}&lang={}", browser, architecture, lang);
    // let url = String::from("http://localhost:8000/firefox-95.0.tar.bz2");

    let workdir = tempfile::Builder::new()
        .prefix("ffui")
        .tempdir()
        .unwrap();
 
    observer(Progress::Status(String::from("Downloading...")));
    let outpath = downloader.download_with_progress(&url, workdir.path(), |current,total| {
        observer(Progress::Percent(current, total));
    })?;

    /* Extract into appdir */
    observer(Progress::Status(String::from("Extracting...")));
    std::fs::create_dir_all(appdir)?;
    let status = process::Command::new("tar")
        .arg("xf")
        .arg(outpath)
        .arg("-C")
        .arg(appdir.as_os_str())
        .status()?;
    if !status.success() {
        return Err(Error::new("Extraction failed"));
    }
    workdir.close();

    let executable = Path::new(appdir).join("firefox/firefox");
    if !executable.exists() {
        return Err(Error::new("Executable not found after extraction"));
    }

    Ok(())
}

fn languages(downloader: &mut Downloader) -> Vec<(String,String)> {
    let mut result = Vec::new();

    let contents = downloader.download_to_string("https://www.mozilla.org/en-US/firefox/all/#product-desktop-release").unwrap();

    let rex0 = Regex::new(r#"(?s)<select id="select_desktop_release_language".*?>.*?</select>"#).unwrap();
    let mat0 = rex0.find(&contents).unwrap();

    let rex1 = Regex::new(r#"<option value="(.*?)">(.*?)</option>"#).unwrap();
    result.push((String::from("en-US"), String::from("English (US)")));
    for mat in rex1.captures_iter(mat0.as_str()) {
        let code = mat.get(1).unwrap().as_str();
        let name = mat.get(2).unwrap().as_str();

        result.push((String::from(code), String::from(name)));
    }

    result
}

fn find_language<'a>(languages: &'a Vec<(String,String)>, lang: &str) -> Option<&'a str> {
    let found = languages.iter().find(|elem| {
        elem.0 == lang
    });

    if let Some(found) = found {
        return Some(&found.0)
    }
    None
}

fn detect_language(languages: &Vec<(String,String)>) -> Option<&str> {
    if let Ok(lang) = env::var("LANG") {
        let lang = lang.split(".").nth(0);
        if let Some(lang) = lang {
            let lang = lang.replace("_", "-");

            /* Find with the form: en-US */
            let found = find_language(languages, &lang);
            if let Some(lang) = found {
                return Some(lang);
            }

            /* If not found, find with the form: en */
            let lang = lang.split("-").next().unwrap();
            return find_language(languages, &lang);
        }
    }
    None
}

fn on_ok(app: Rc<Application>, browser: &str, architecture: &str, lang: &str) {
    let (tx, rx) = glib::MainContext::channel(glib::PRIORITY_DEFAULT);

    let window = Window::builder()
        .application(app.as_ref())
        .title("Installing")
        .default_width(350)
        .default_height(56)
        .window_position(gtk::WindowPosition::Center)
        .build();
    
    let grid = Grid::builder()
        .column_spacing(5)
        .row_spacing(5)
        .margin(5)
        .build();
    window.add(&grid);

    let progressbar = ProgressBar::builder()
        .show_text(true)
        .text("Installing...")
        .expand(true)
        .build();
    grid.attach(&progressbar, 0, 0, 1, 1);
    window.show_all();

    rx.attach(None, move |event| {
        match event {
            InstallEvent::Progress(progress) => match progress {
                Progress::Status(text) => {
                    progressbar.set_text(Some(&text));
                }
                Progress::Percent(current, total) => {
                    if total != -1 {
                        let fraction = current as f64 / total as f64;
                        progressbar.set_fraction(fraction);
                    } else {
                        progressbar.pulse();
                    }
                }
            }

            InstallEvent::Success => {
                window.close();
                run_app();
                return Continue(false);
            }
            InstallEvent::Error(err) => {
                let dlg = MessageDialog::new(
                    Some(&window),
                    DialogFlags::MODAL,
                    MessageType::Error,
                    ButtonsType::Ok,
                    &err.to_string());
                dlg.run();
                dlg.close();
                window.close();
                return Continue(false);
            }
        }
        Continue(true)
    });

    let browser = String::from(browser);
    let architecture = String::from(architecture);
    let lang = String::from(lang);
    thread::spawn(move || {
        let datadir = get_datadir();
        let appdir = Path::new(&datadir).join("app");
        let result = install(&datadir, &appdir, &browser, &architecture, &lang, |progress| {
            tx.send(InstallEvent::Progress(progress));
        });

        match result {
            Ok(_) => {
                tx.send(InstallEvent::Success);
            }
            Err(e) => {
                tx.send(InstallEvent::Error(e));
            }
        }
    });

    
}

fn run_app() {
    let datadir = get_datadir();
    let appdir = Path::new(&datadir).join("app");
    let exe = Path::new(&appdir).join("firefox/firefox");

    let mut proc = process::Command::new(exe);
    for arg in env::args().skip(1) {
        if arg != "--reset" {
            proc.arg(arg);
        }
    }
    
    let error = proc.exec();
    let dlg = MessageDialog::new::<MessageDialog>(
        None,
        DialogFlags::MODAL,
        MessageType::Error,
        ButtonsType::Ok,
        &format!("Cannot launch firefox: {}", &error.to_string()));
    dlg.set_window_position(gtk::WindowPosition::Center);
    dlg.run();
    dlg.close();
}

fn build_ui(app: Rc<Application>) {
    let (tx, rx) = glib::MainContext::channel(glib::PRIORITY_DEFAULT);

    let window = Rc::new(ApplicationWindow::builder()
        .application(app.as_ref())
        .title("Firefox User Installer")
        .default_width(350)
        .default_height(70)
        .window_position(gtk::WindowPosition::Center)
        .build());

    let grid = Grid::builder()
        .border_width(5)
        .column_spacing(5)
        .row_spacing(5)
        .build();
    window.add(&grid);

    let label = Label::builder()
        .label("Browser:")
        .build();
    grid.attach(&label, 0, 0, 1, 1);

    let mut browser_combo = ComboBoxText::builder()
        .build();
    browser_combo.append(Some("firefox-latest-ssl"), "Firefox");
    browser_combo.append(Some("firefox-beta-latest-ssl"), "Firefox Beta");
    browser_combo.append(Some("firefox-devedition-latest-ssl"), "Firefox Developer Edition");
    browser_combo.append(Some("firefox-nightly-latest-l10n-ssl"), "Firefox Nightly");
    browser_combo.append(Some("firefox-esr-latest-ssl"), "Firefox Extended Support Release");
    browser_combo.set_active_id(Some("firefox-latest-ssl"));
    grid.attach(&browser_combo, 1, 0, 1, 1);

    let label = Label::builder()
        .label("Architecture:")
        .build();
    grid.attach(&label, 0, 1, 1, 1);

    let mut architecture_combo = ComboBoxText::builder().build();
    architecture_combo.append(Some("linux64"), "Linux 64-bit");
    architecture_combo.append(Some("linux"), "Linux 32-bit");
    match env::consts::ARCH {
        "x86_64" => {
            architecture_combo.set_active_id(Some("linux64"));
        }
        _ => {
            architecture_combo.set_active_id(Some("linux"));
        }
    }
    grid.attach(&architecture_combo, 1, 1, 1, 1);

    let label = Label::builder()
        .label("Language:")
        .build();
    grid.attach(&label, 0, 2, 1, 1);
    
    let language_combo = Rc::new(ComboBoxText::new());
    grid.attach(language_combo.as_ref(), 1, 2, 1, 1);
    thread::spawn(move || {
        let datadir = get_datadir();
        let cachedir = get_cachedir(&datadir);
        let mut downloader = Downloader::new(&cachedir);
        let languages = languages(&mut downloader);
        tx.send(languages);
    });

    let ok_button = Rc::new(Button::builder()
        .label("OK")
        .sensitive(false)
        .build());
    ok_button.connect_clicked(clone!(@strong app, @strong window, @strong language_combo, @strong ok_button => move |_| {
        let browser = browser_combo.active_id().unwrap();
        let architecture = architecture_combo.active_id().unwrap();
        let language = language_combo.active_id().unwrap();

        on_ok(Rc::clone(&app), &browser, &architecture, &language);

        window.close();
    }));
    grid.attach(ok_button.as_ref(), 0, 3, 2, 1);

    rx.attach(None, clone!(@strong language_combo, @strong ok_button => move |languages| {
        for lang in languages.iter() {
            language_combo.append(Some(&lang.0), &lang.1);
        }

        let detected_language = detect_language(&languages);
        language_combo.set_active_id(Some(detected_language.unwrap_or("en-US")));
        ok_button.set_sensitive(true);
        Continue(false)
    }));

    window.show_all();
}

fn main() {
    let args: Vec<String> = env::args().collect();

    let datadir = get_datadir();
    let cachedir = get_cachedir(&datadir);
    let appdir = Path::new(&datadir).join("app");
    let exe = Path::new(&appdir).join("firefox/firefox");
    let reset = env::args().skip(1).find(|s| s == "--reset").is_some();

    let application = Rc::new(Application::builder()
        .application_id("io.degaart.firefox-user-installer")
        .build());

    application.connect_activate(clone!(@strong application => move |_| {
        if reset || !exe.exists() {
            build_ui(Rc::clone(&application));
        } else {
            run_app();
        }
    }));

    let args: &[&str] = &[];
    application.run_with_args(args);
}

