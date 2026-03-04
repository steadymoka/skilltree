use std::collections::BTreeMap;
use std::fs;
use std::path::Path;
use std::time::SystemTime;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

const LOCK_VERSION: u32 = 1;

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct SkillLock {
    pub version: u32,
    pub skills: BTreeMap<String, SkillLockEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SkillLockEntry {
    pub source: String,
    pub skill_path: String,
    pub git_ref: String,
    pub installed_sha: String,
    pub installed_at: String,
}

impl Default for SkillLock {
    fn default() -> Self {
        Self {
            version: LOCK_VERSION,
            skills: BTreeMap::new(),
        }
    }
}

impl SkillLock {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn get(&self, name: &str) -> Option<&SkillLockEntry> {
        self.skills.get(name)
    }

    pub fn insert(&mut self, name: String, entry: SkillLockEntry) {
        self.skills.insert(name, entry);
    }

    pub fn remove(&mut self, name: &str) -> Option<SkillLockEntry> {
        self.skills.remove(name)
    }
}

impl SkillLockEntry {
    pub fn new(source: &str, skill_path: &str, git_ref: &str, sha: &str) -> Self {
        Self {
            source: source.to_string(),
            skill_path: skill_path.to_string(),
            git_ref: git_ref.to_string(),
            installed_sha: sha.to_string(),
            installed_at: iso_now(),
        }
    }
}

pub fn read_lock(path: &Path) -> Result<SkillLock> {
    match fs::read_to_string(path) {
        Ok(content) => {
            let lock: SkillLock = match serde_json::from_str(&content) {
                Ok(l) => l,
                Err(_) => return Ok(SkillLock::new()),
            };
            if lock.version != LOCK_VERSION {
                return Ok(SkillLock::new());
            }
            Ok(lock)
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(SkillLock::new()),
        Err(e) => Err(anyhow::Error::from(e).context("failed to read .skill-lock.json")),
    }
}

pub fn write_lock(path: &Path, lock: &SkillLock) -> Result<()> {
    let content =
        serde_json::to_string_pretty(lock).context("failed to serialize .skill-lock.json")?;
    fs::write(path, content).context("failed to write .skill-lock.json")?;
    Ok(())
}

fn iso_now() -> String {
    let dur = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default();
    let secs = dur.as_secs();
    // Simple UTC timestamp without chrono
    let (days, remainder) = (secs / 86400, secs % 86400);
    let (hours, remainder) = (remainder / 3600, remainder % 3600);
    let (minutes, seconds) = (remainder / 60, remainder % 60);

    // Days since epoch to Y-M-D (simplified)
    let mut y = 1970i64;
    let mut remaining_days = days as i64;
    loop {
        let year_days = if is_leap(y) { 366 } else { 365 };
        if remaining_days < year_days {
            break;
        }
        remaining_days -= year_days;
        y += 1;
    }
    let month_days: &[i64] = if is_leap(y) {
        &[31, 29, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    } else {
        &[31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    };
    let mut m = 1;
    for &md in month_days {
        if remaining_days < md {
            break;
        }
        remaining_days -= md;
        m += 1;
    }
    let d = remaining_days + 1;

    format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z",
        y, m, d, hours, minutes, seconds
    )
}

fn is_leap(y: i64) -> bool {
    (y % 4 == 0 && y % 100 != 0) || y % 400 == 0
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn roundtrip_lock() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join(".skill-lock.json");

        let mut lock = SkillLock::new();
        lock.insert(
            "my-skill".into(),
            SkillLockEntry::new("user/repo", "skills/my-skill", "main", "abc123"),
        );

        write_lock(&path, &lock).unwrap();
        let loaded = read_lock(&path).unwrap();

        assert_eq!(loaded.version, LOCK_VERSION);
        assert_eq!(loaded.skills.len(), 1);
        let entry = loaded.get("my-skill").unwrap();
        assert_eq!(entry.source, "user/repo");
        assert_eq!(entry.installed_sha, "abc123");
    }

    #[test]
    fn read_missing_returns_empty() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("nonexistent.json");
        let lock = read_lock(&path).unwrap();
        assert!(lock.skills.is_empty());
        assert_eq!(lock.version, LOCK_VERSION);
    }

    #[test]
    fn read_incompatible_version_returns_fresh() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join(".skill-lock.json");
        fs::write(&path, r#"{"version": 999, "skills": {"old": {}}}"#).unwrap();

        let lock = read_lock(&path).unwrap();
        assert!(lock.skills.is_empty());
        assert_eq!(lock.version, LOCK_VERSION);
    }

    #[test]
    fn read_corrupted_json_returns_fresh() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join(".skill-lock.json");
        fs::write(&path, "not json{{").unwrap();

        let lock = read_lock(&path).unwrap();
        assert!(lock.skills.is_empty());
        assert_eq!(lock.version, LOCK_VERSION);
    }

    #[test]
    fn insert_and_remove() {
        let mut lock = SkillLock::new();
        lock.insert("a".into(), SkillLockEntry::new("u/r", ".", "main", "sha1"));
        assert!(lock.get("a").is_some());

        lock.remove("a");
        assert!(lock.get("a").is_none());
    }

    #[test]
    fn iso_now_format() {
        let ts = iso_now();
        // Should match YYYY-MM-DDTHH:MM:SSZ
        assert!(ts.ends_with('Z'));
        assert_eq!(ts.len(), 20);
        assert_eq!(&ts[4..5], "-");
        assert_eq!(&ts[7..8], "-");
        assert_eq!(&ts[10..11], "T");
    }
}
