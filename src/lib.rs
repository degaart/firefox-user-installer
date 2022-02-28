#![allow(unused)]
#![allow(dead_code)]

use chrono::{Utc, Duration, DateTime, NaiveDateTime};
use json::JsonValue;
use json::object;
use reqwest::blocking::Client;
use reqwest::header::{ HeaderMap, HeaderValue} ;
use std::collections::HashMap;
use std::fs::File;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use indicatif::{ProgressBar, ProgressStyle};

struct CacheEntry {
    filename: String,
    date: DateTime<Utc>,
}

impl CacheEntry {
    fn new(filename: &str) -> Self {
        Self {
            filename: String::from(filename),
            date: Utc::now()
        }
    }
}

impl From<&JsonValue> for CacheEntry {
    fn from(value: &JsonValue) -> Self {
        match value {
            JsonValue::Object(obj) => {
                let filename = String::from(obj["file"].as_str().unwrap());
                let date = DateTime::from_utc(NaiveDateTime::from_timestamp(obj["date"].as_i64().unwrap(), 0), Utc);
                CacheEntry { filename, date }
            }

            _ => {
                panic!("Invalid value type");
            }
        }
    }
}

pub struct Downloader {
    cachedir: PathBuf,
    index_file: PathBuf,
    cache: HashMap<String,CacheEntry>,
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

    fn load_cache(index_file: &Path) -> HashMap<String,CacheEntry> {
        let mut result: HashMap<String,CacheEntry> = HashMap::new();

        if let Ok(mut f) = File::open(index_file) {
            let mut contents = String::new();
            if let Ok(_) = f.read_to_string(&mut contents) {
                if let Ok(cache_data) = json::parse(&contents) {
                    if let JsonValue::Object(obj) = cache_data {
                        for (k, v) in obj.iter() {
                            let entry = CacheEntry::from(v);
                            result.insert(String::from(k), entry);
                        }
                    }
                }
            }
        }

        result
    }

    fn save_cache(index_file: &Path, cache: &HashMap<String,CacheEntry>) {
        let mut data = object!{};
        for (k, entry) in cache {
            let jentry = object! {
                "file": entry.filename.clone(),
                "date": entry.date.timestamp()
            };
            data.insert(k, jentry);
        }
        let mut f = File::create(index_file).expect("Failed to write cache file");
        f.write_all(data.dump().as_bytes()).expect("Failed to write cache file");
    }

    fn get_cache_entry(&mut self, url: &str) -> Option<PathBuf> {
        let url = Self::normalize_url(url);
        let entry = self.cache.get(&url)?;
        let now = Utc::now();

        if Utc::now() - entry.date > Duration::hours(1) {
            self.remove_cache_entry(&url);
            None
        } else {
            let mut result = PathBuf::from(&self.cachedir);
            result.push(entry.filename.clone());
            Some(result)
        }
    }

    fn add_cache_entry(&mut self, url: &str, file: &str) {
        let entry = CacheEntry::new(file);
        self.cache.insert(String::from(url), entry);
        Self::save_cache(&self.index_file, &self.cache);
    }

    fn remove_cache_entry(&mut self, url: &str) {
        let url = Self::normalize_url(url);
        let entry = self.cache.remove(&url);
        if let Some(entry) = entry {
            let file = Path::new(&self.cachedir).join(entry.filename);
            std::fs::remove_file(file);
        }
    }

    pub fn download_to_string(&mut self, url: &str) -> Option<String> {
        let cache_entry = self.get_cache_entry(url);
        if let Some(path) = cache_entry {
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

        let result = req.send().ok()?.text().ok()?;
        let mut cachefile = tempfile::Builder::new()
            .prefix("c_")
            .suffix(".cache")
            .tempfile_in(&self.cachedir)
            .ok()?;
        let mut f = cachefile.as_file_mut();
        f.write_all(result.as_bytes());

        let filename = cachefile.path().file_name().unwrap().to_str().unwrap();
        self.add_cache_entry(url, filename);
        cachefile.keep();
        
        Some(result)
    }

    /*
        This does not use a cache (but we still let the user manually add a cache entry)
    */
    pub fn download(&mut self, url: &str, destdir: &Path) -> Option<PathBuf> {
        let cache_entry = self.get_cache_entry(url);
        if let Some(_) = cache_entry {
            return cache_entry;
        }

        let client = Client::new();
        let mut req = client.get(url);

        let mut response = req.send().ok()?;
        let mut filename = String::from("");
        let url_filename = response
            .url()
            .path_segments()
            .and_then(|segments| segments.last())
            .and_then(|name| if name.is_empty() { None } else { Some(name) });

        let filename = match url_filename {
            None => {
                let tmpfile = tempfile::Builder::new()
                    .prefix("download_")
                    .tempfile_in(destdir)
                    .ok()?;
                let tmpfile = tmpfile.path();
                let tmpfile = tmpfile.to_str().unwrap_or("outfile");
                let tmpfile = String::from(tmpfile);
                String::from(tmpfile)
            }

            Some(filename) => String::from(filename)
        };
        
        let out_path = Path::new(destdir).join(filename);

        let mut outfile = File::create(&out_path).ok()?;
        let content_length = response.content_length();

        // let content_length = None;
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
        
        let mut buf = [ 0u8; 65536 ];
        while let Ok(len) = response.read(&mut buf) {
            if len == 0 {
                break;
            }
            outfile.write_all(&buf[0..len]);

            progress.inc(len as u64);
        }
        progress.finish();
        Some(out_path)
    }
}
