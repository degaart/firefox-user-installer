#![allow(unused)]
#![allow(dead_code)]

use chrono::Duration;
use chrono::prelude::*;
use ffuinstaller::Downloader;
use ffuinstaller::Error;
use regex::Regex;
use std::collections::HashMap;
use std::os::unix::process::CommandExt;
use std::path::{ Path, PathBuf };
use std::process;

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

fn install(datadir: &Path, appdir: &Path, browser: &str, installer: &str, language: &str) -> Result<(),Error> {
    let cachedir = get_cachedir(datadir);
    let mut downloader = Downloader::new(&cachedir);

    let url = format!("https://download.mozilla.org/?product={}&os={}&lang={}", browser, installer, language);
    // let url = String::from("http://localhost:8000/firefox-95.0.tar.bz2");

    let workdir = tempfile::Builder::new()
        .prefix("ffui")
        .tempdir()
        .unwrap();
 
    let outpath = downloader.download(&url, workdir.path()).unwrap();

    /* Extract into appdir */
    println!("Extracting...");
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

fn main() {
    let datadir = get_datadir();
    let cachedir = get_cachedir(&datadir);
    let appdir = Path::new(&datadir).join("app");
    let exe = Path::new(&appdir).join("firefox/firefox");

    if !exe.exists() {
        /* Browser to download */
        let mut browser: HashMap<String,String> = HashMap::new();
        browser.insert(String::from("Firefox"), String::from("firefox-latest-ssl"));
        browser.insert(String::from("Firefox Beta"), String::from("firefox-beta-latest-ssl"));
        browser.insert(String::from("Firefox Developer Edition"), String::from("firefox-devedition-latest-ssl"));
        browser.insert(String::from("Firefox Nightly"), String::from("firefox-nightly-latest-l10n-ssl"));
        browser.insert(String::from("Firefox Extended Support Release"), String::from("firefox-esr-latest-ssl"));

        /* Prefered installer */
        let mut installer: HashMap<String,String> = HashMap::new();
        installer.insert(String::from("Linux 32-bit"), String::from("linux"));
        installer.insert(String::from("Linux 64-bit"), String::from("linux64"));
        
        /* Language */
        let cachedir = get_cachedir(&datadir);
        let mut downloader = Downloader::new(&cachedir);
        let contents = downloader.download_to_string("https://www.mozilla.org/en-US/firefox/all/#product-desktop-release").unwrap();

        let mut language: HashMap<String,String> = HashMap::new();
        let rex0 = Regex::new(r#"(?s)<select id="select_desktop_release_language".*?>.*?</select>"#).unwrap();
        let mat0 = rex0.find(&contents).unwrap();

        let rex1 = Regex::new(r#"<option value="(.*?)">(.*?)</option>"#).unwrap();
        language.insert(String::from("English (US)"), String::from("en-US"));
        for mat in rex1.captures_iter(mat0.as_str()) {
            let code = mat.get(1).unwrap().as_str();
            let name = mat.get(2).unwrap().as_str();
            language.insert(String::from(name), String::from(code));
        }
        
        install(&datadir, &appdir, &browser["Firefox"], &installer["Linux 64-bit"], &language["English (US)"]);
    }

    let error = process::Command::new(exe).exec();
    panic!("Error: {:?}", error);
}

