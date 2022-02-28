#![allow(unused)]
#![allow(dead_code)]

use std::path::{ Path, PathBuf };
use chrono::prelude::*;
use chrono::Duration;
use ffuinstaller::Downloader;
use regex::Regex;
use std::collections::HashMap;

fn get_datadir() -> PathBuf {
    let dirs = directories::ProjectDirs::from("", "", "firefox-user-installer").unwrap();
    let result = PathBuf::from(dirs.data_dir());
    if !result.exists() {
        std::fs::create_dir_all(&result)
            .unwrap_or_else(|e| panic!("Failed to create directory {}: {}", result.display(), e));
    }
    result
}

fn get_cachedir(datadir: &PathBuf) -> PathBuf {
    let result: PathBuf = [datadir, Path::new("cache")].iter().collect();
    if !result.exists() {
        std::fs::create_dir_all(&result)
            .unwrap_or_else(|e| panic!("Failed to create directory {}: {}", result.display(), e));
    }
    result
}

fn main() {
    let cache_dir = get_cachedir(&get_datadir());
    println!("cache_dir: {:?}", cache_dir);
    
    let mut downloader = Downloader::new(&cache_dir);
    let contents = downloader.download_to_string("https://www.mozilla.org/en-US/firefox/all/#product-desktop-release").unwrap();

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

    let url = format!(
        "https://download.mozilla.org/?product={product}&os={os}&lang={lang}",
        product="firefox-latest-ssl",
        os="linux",
        lang="en-US");
    let url = String::from("http://localhost:8000/firefox-95.0.tar.bz2");
    
    let outpath = downloader.download(&url, Path::new("/tmp")).unwrap();
    println!("{}", outpath.display());
}

