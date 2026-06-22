use anyhow::{Context, Result};
use chrono::{LocalResult, TimeZone, Utc};
use serde::{Deserialize, Serialize};
use std::ffi::OsStr;
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Package {
    pub name: String,
    pub version: String,
    pub description: String,
    pub size: u64,
    pub install_date_ts: u64,
    pub has_desktop: bool,
    pub bins: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Category {
    App,
    Cli,
    Library,
}

impl Package {
    pub fn category(&self) -> Category {
        if self.has_desktop {
            Category::App
        } else if !self.bins.is_empty() {
            Category::Cli
        } else {
            Category::Library
        }
    }

    pub fn size_human(&self) -> String {
        if self.size == 0 {
            return "0 B".into();
        }
        let units = ["B", "KB", "MB", "GB", "TB"];
        let bytes = self.size as f64;
        let i = (bytes.log10() / 3.0).floor() as usize;
        let i = i.min(units.len() - 1);
        let val = bytes / (1024u64.pow(i as u32) as f64);
        format!("{:.1} {}", val, units[i])
    }

    pub fn date_formatted(&self) -> String {
        if self.install_date_ts == 0 {
            return "unknown".into();
        }
        match Utc.timestamp_opt(self.install_date_ts as i64, 0) {
            LocalResult::Single(dt) => dt.format("%Y-%m-%d").to_string(),
            _ => "unknown".into(),
        }
    }

    pub fn type_label(&self) -> String {
        match self.category() {
            Category::App if !self.bins.is_empty() => "App + CLI".into(),
            Category::App => "App".into(),
            Category::Cli => "CLI".into(),
            Category::Library => "Library".into(),
        }
    }
}

const PACMAN_DB: &str = "/var/lib/pacman/local";

pub fn db_mtime() -> Result<u64> {
    let m = fs::metadata(PACMAN_DB).context("cannot access pacman database")?;
    m.modified()
        .map(|t| t.duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_secs())
        .context("cannot read mtime")
}

pub fn scan() -> Result<Vec<Package>> {
    let db_dir = Path::new(PACMAN_DB);
    let entries = fs::read_dir(db_dir).context("cannot read pacman database directory")?;

    let mut packages = Vec::new();

    for entry in entries {
        let entry = entry?;
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }

        let desc_path = path.join("desc");
        if !desc_path.exists() {
            continue;
        }

        let files_path = path.join("files");

        let desc = parse_desc(&desc_path);
        let (has_desktop, bins) = if files_path.exists() {
            parse_files(&files_path)
        } else {
            (false, Vec::new())
        };

        if let Ok(pkg) = desc {
            packages.push(Package {
                has_desktop,
                bins,
                ..pkg
            });
        }
    }

    packages.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(packages)
}

fn parse_desc(path: &Path) -> Result<Package> {
    let content = fs::read_to_string(path).context("cannot read desc file")?;
    let raw = content.replace("\r\n", "\n");
    let lines: Vec<&str> = raw.lines().collect();

    let mut name = String::new();
    let mut version = String::new();
    let mut description = String::new();
    let mut size = 0u64;
    let mut install_date = 0u64;

    let mut i = 0;
    while i < lines.len() {
        let line = lines[i].trim();
        match line {
            "%NAME%" | "%NAME%\r" => {
                if let Some(v) = lines.get(i + 1).map(|s| s.trim()).filter(|s| !s.is_empty()) {
                    name = v.to_string();
                    i += 2;
                    continue;
                }
            }
            "%VERSION%" | "%VERSION%\r" => {
                if let Some(v) = lines.get(i + 1).map(|s| s.trim()).filter(|s| !s.is_empty()) {
                    version = v.to_string();
                    i += 2;
                    continue;
                }
            }
            "%DESC%" | "%DESC%\r" => {
                if let Some(v) = lines.get(i + 1).map(|s| s.trim()).filter(|s| !s.is_empty()) {
                    description = v.to_string();
                    i += 2;
                    continue;
                }
            }
            "%SIZE%" | "%SIZE%\r" => {
                if let Some(v) = lines.get(i + 1).map(|s| s.trim()).filter(|s| !s.is_empty()) {
                    size = v.parse().unwrap_or(0);
                    i += 2;
                    continue;
                }
            }
            "%INSTALLDATE%" | "%INSTALLDATE%\r" => {
                if let Some(v) = lines.get(i + 1).map(|s| s.trim()).filter(|s| !s.is_empty()) {
                    install_date = v.parse().unwrap_or(0);
                    i += 2;
                    continue;
                }
            }
            _ => {}
        }
        i += 1;
    }

    Ok(Package {
        name: name_or_fallback(&name, &path),
        version,
        description,
        size,
        install_date_ts: install_date,
        has_desktop: false,
        bins: Vec::new(),
    })
}

fn name_or_fallback(name: &str, path: &Path) -> String {
    if !name.is_empty() {
        return name.to_string();
    }
    path.file_name()
        .and_then(OsStr::to_str)
        .map(|s| {
            s.rsplit_once('-')
                .or_else(|| s.rsplit_once(':'))
                .map(|(n, _)| n.to_string())
                .unwrap_or_else(|| s.to_string())
        })
        .unwrap_or_else(|| "unknown".to_string())
}

fn parse_files(path: &Path) -> (bool, Vec<String>) {
    let content = match fs::read_to_string(path) {
        Ok(c) => c,
        Err(_) => return (false, Vec::new()),
    };
    let raw = content.replace("\r\n", "\n");
    let lines: Vec<&str> = raw.lines().collect();

    let mut in_files = false;
    let mut has_desktop = false;
    let mut bins = Vec::new();

    for &line in &lines {
        let trimmed = line.trim();
        if trimmed == "%FILES%" {
            in_files = true;
            continue;
        }
        if !in_files || trimmed.is_empty() || trimmed.starts_with('%') {
            if !trimmed.is_empty() && trimmed.starts_with('%') {
                in_files = false;
            }
            continue;
        }

        let entry = trimmed;
        if entry.ends_with('/') {
            continue;
        }

        let bin_dirs = ["usr/bin/", "usr/local/bin/", "usr/sbin/"];
        for dir in &bin_dirs {
            if entry.starts_with(dir) {
                let bin_name = entry.strip_prefix(dir).unwrap_or(entry);
                if !bin_name.is_empty() && !bin_name.contains('/') {
                    bins.push(format!("/{}{}", dir, bin_name));
                }
                break;
            }
        }

        if entry.starts_with("usr/share/applications/") && entry.ends_with(".desktop") {
            has_desktop = true;
        }
    }

    bins.sort();
    bins.dedup();
    (has_desktop, bins)
}

pub fn packages_by_category(packages: &[Package]) -> (usize, usize, usize) {
    let apps = packages.iter().filter(|p| p.has_desktop).count();
    let clis = packages.iter().filter(|p| !p.bins.is_empty()).count();
    let libs = packages
        .iter()
        .filter(|p| !p.has_desktop && p.bins.is_empty())
        .count();
    (apps, clis, libs)
}
