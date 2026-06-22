use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

use crate::db::{db_mtime, Package};

#[derive(Serialize, Deserialize)]
struct CacheData {
    db_mtime: u64,
    packages: Vec<Package>,
}

fn cache_path() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".into());
    let dir = PathBuf::from(home).join(".cache").join("woler");
    let _ = fs::create_dir_all(&dir);
    dir.join("packages.msgpack")
}

pub fn load() -> Result<Option<Vec<Package>>> {
    let path = cache_path();
    if !path.exists() {
        return Ok(None);
    }

    let current_mtime = db_mtime()?;
    let data = fs::read(&path)?;
    let cached: CacheData = rmp_serde::from_slice(&data)?;

    if cached.db_mtime == current_mtime {
        Ok(Some(cached.packages))
    } else {
        Ok(None)
    }
}

pub fn save(packages: &[Package]) -> Result<()> {
    let mtime = db_mtime()?;
    let data = CacheData {
        db_mtime: mtime,
        packages: packages.to_vec(),
    };
    let bytes = rmp_serde::to_vec(&data)?;
    fs::write(cache_path(), &bytes)?;
    Ok(())
}

pub fn clear() -> Result<()> {
    let path = cache_path();
    if path.exists() {
        fs::remove_file(&path)?;
    }
    Ok(())
}
