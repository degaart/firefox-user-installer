#![allow(unused)]
#![allow(dead_code)]

use chrono::{Utc, Duration, DateTime, NaiveDateTime};
use indicatif::{ProgressBar, ProgressStyle};
use json::JsonValue;
use json::object;
use reqwest::blocking::Response;
use reqwest::blocking::Client;
use reqwest::header::{ HeaderMap, HeaderValue} ;
use std::collections::HashMap;
use std::fmt;
use std::fs::File;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use serde::{Deserialize, Serialize};
use serde_json::Result;

#[derive(Debug, Clone)]
pub struct Error {
    message: String,
}

impl Error {
    pub fn new(message: &str) -> Self {
        Self { message: String::from(message) }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        Self { message: e.to_string() }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct CacheEntry {
    cache_file: String,
    date: i64,
    max_age: i64,
    filename: String,
}

// DateTime::from_utc(NaiveDateTime::from_timestamp(obj["date"].as_i64().unwrap(), 0), Utc);
impl CacheEntry {
    fn new(cache_file: &str, max_age: i64, filename: &str) -> Self {
        Self {
            cache_file : String::from(cache_file),
            date: Utc::now().timestamp(),
            max_age: max_age,
            filename: String::from(filename),
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
struct Cache {
    cache: HashMap<String,CacheEntry>,
}

impl Cache {
    fn new() -> Cache {
        Cache { cache: HashMap::new() }
    }
}

pub struct Downloader {
    cachedir: PathBuf,
    index_file: PathBuf,
    cache: Cache,
}

impl Downloader {

    pub fn new(cachedir: &PathBuf) -> Self {
        let index_file: PathBuf =  [ cachedir, &PathBuf::from("cache.json") ].iter().collect();
        let cache = Self::load_cache(&index_file);
        Self { cachedir: cachedir.clone(), index_file, cache }
    }
    
    fn normalize_url(url: &str) -> String {
        let mut url = String::from(url);
        while url.ends_with("/") {
            url.pop();
        }
        url
    }

    fn load_cache(index_file: &Path) -> Cache {
        let mut result = Cache::new();

        if let Ok(mut f) = File::open(index_file) {
            if let Ok(cache) = serde_json::from_reader(f) {
                result = cache;
            }
        }

        result
    }

    fn save_cache(index_file: &Path, cache: &Cache) {
        let mut f = File::create(index_file).expect("Failed to write cache file");
        serde_json::to_writer(f, cache);
    }

    fn get_cache_entry(&mut self, url: &str) -> Option<CacheEntry> {
        let url = Self::normalize_url(url);
        let entry = self.cache.cache.get(&url)?;
        let now = Utc::now();

        let entry_date = DateTime::from_utc(NaiveDateTime::from_timestamp(entry.date, 0), Utc);
        let max_age = Duration::seconds(entry.max_age);
        if Utc::now() - entry_date > max_age {
            self.remove_cache_entry(&url);
            None
        } else {
            Some(entry.clone())
        }
    }

    fn add_cache_entry(&mut self, url: &str, file: &Path, max_age: i64, filename: &str) {
        let url = Self::normalize_url(url);
        let cache_file = String::from(file.file_name().unwrap().to_str().unwrap());
        self.cache.cache.insert(url, CacheEntry::new(&cache_file, max_age, filename));
        Self::save_cache(&self.index_file, &self.cache);
    }

    fn remove_cache_entry(&mut self, url: &str) {
        let url = Self::normalize_url(url);
        let entry = self.cache.cache.remove(&url);
        if let Some(entry) = entry {
            let file = Path::new(&self.cachedir).join(entry.cache_file);
            std::fs::remove_file(file);
        }
    }

    fn get_filename(r: &Response) -> Option<String> {
        match r
            .url()
            .path_segments()
            .and_then(|segments| segments.last())
            .and_then(|name| if name.is_empty() { None } else { Some(name) }) {
                None => None,
                Some(s) => Some(String::from(s))
        }
    }

    pub fn download_to_string(&mut self, url: &str) -> Option<String> {
        let cache_entry = self.get_cache_entry(url);
        if let Some(cache_entry) = cache_entry {
            let path = Path::new(&self.cachedir).join(&cache_entry.cache_file);
            match File::open(path) {
                Ok(mut f) => {
                    let mut contents = String::new();
                    match f.read_to_string(&mut contents) {
                        Ok(_) => {
                            return Some(contents);
                        }

                        Err(_) => {
                            self.remove_cache_entry(url);
                        }
                    }
                }

                Err(_) => {
                    self.remove_cache_entry(url);
                }
            }
        }

        let client = Client::new();
        let mut req = client.get(url);
        let response = req.send().ok()?;
        let filename = Self::get_filename(&response);
        let result = response.text().ok()?;
        let mut cachefile = tempfile::Builder::new()
            .prefix("c_")
            .suffix(".cache")
            .tempfile_in(&self.cachedir)
            .ok()?;
        let mut f = cachefile.as_file_mut();
        f.write_all(result.as_bytes());

        self.add_cache_entry(url, cachefile.path(), 3600, &filename.unwrap_or(String::from("download")));
        cachefile.keep();

        Some(String::from(result))
    }

    /*
        This does not use a cache (but we still let the user manually add a cache entry)
    */
    pub fn download(&mut self, url: &str, destdir: &Path) -> Option<PathBuf> {
        let cache_entry = self.get_cache_entry(url);
        if let Some(cache_entry) = cache_entry {
            let cache_file = Path::new(&self.cachedir).join(&cache_entry.cache_file);
            if !cache_file.exists() {
                self.remove_cache_entry(url);
            }

            let dest_file = Path::new(destdir).join(&cache_entry.filename);
            if let Ok(_) = std::fs::copy(cache_file, &dest_file) {
                return Some(dest_file);
            }
        }

        let client = Client::new();
        let mut req = client.get(url);
        let mut response = req.send().ok()?;
        let content_length = response.content_length();
        let mut tmpfile = tempfile::Builder::new()
            .prefix("download_")
            .tempfile_in(&self.cachedir)
            .ok()?;
        let progress = match content_length {
            Some(len) => {
                let pb = ProgressBar::new(len);
                pb.set_style(
                    ProgressStyle::default_bar()
                    .template("[{elapsed_precise}] {bar} {bytes}/{total_bytes} {binary_bytes_per_sec} {percent}% [{eta_precise}]"));
                pb
            }
            None => {
                let pb = ProgressBar::new_spinner();
                    pb.set_style(
                        ProgressStyle::default_spinner()
                        .template("{spinner} [{elapsed_precise}] {bytes}")
                );
                pb
            }
        };
            
        let mut outfile = tmpfile.as_file_mut();
        let mut buf = [ 0u8; 65536 ];
        while let Ok(len) = response.read(&mut buf) {
            if len == 0 {
                break;
            }
            outfile.write_all(&buf[0..len]);

            progress.inc(len as u64);
        }
        progress.finish();

        let filename = Self::get_filename(&response);
        let tmp_path = tmpfile.path().to_path_buf();
        self.add_cache_entry(url, &tmp_path, 30i64 * 24i64 * 3600i64, &filename.clone().unwrap_or(String::from("download")));
        tmpfile.keep();

        let dest_file = match filename {
            Some(filename) => {
                PathBuf::from(destdir).join(&filename)
            }

            None => {
                let tmpfile = tempfile::Builder::new()
                    .prefix("download_")
                    .tempfile_in(destdir)
                    .ok()?;
                let result = tmpfile.path().to_path_buf();
                tmpfile.keep();
                result
            }
        };

        std::fs::copy(&tmp_path, &dest_file);
        Some(dest_file.to_path_buf())
    }
}
