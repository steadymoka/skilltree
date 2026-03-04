use std::path::PathBuf;

use anyhow::{Context, Result};

#[derive(Debug, Clone)]
pub struct Paths {
    /// Legacy ~/.claude/skills (migration source)
    pub skills_dir: PathBuf,
    /// ~/.skilltree/
    pub skill_tree_dir: PathBuf,
    pub skills_yaml: PathBuf,
    pub link_skills_sh: PathBuf,
    pub skill_lock_json: PathBuf,
}

impl Paths {
    pub fn from_home(home: &std::path::Path) -> Self {
        let skill_tree = home.join(".skilltree");
        let skills_yaml = skill_tree.join("skills.yaml");
        let link_skills_sh = skill_tree.join("link-skills.sh");
        let skill_lock_json = skill_tree.join(".skill-lock.json");
        Self {
            skills_dir: home.join(".claude").join("skills"),
            skill_tree_dir: skill_tree,
            skills_yaml,
            link_skills_sh,
            skill_lock_json,
        }
    }

    pub fn default_paths() -> Result<Self> {
        let home = dirs::home_dir().context("cannot determine home directory")?;
        Ok(Self::from_home(&home))
    }
}

/// Load project paths from ~/.claude.json.
pub fn load_project_paths() -> Vec<String> {
    let home = match dirs::home_dir() {
        Some(h) => h,
        None => return Vec::new(),
    };
    let claude_json = home.join(".claude.json");
    let content = match std::fs::read_to_string(&claude_json) {
        Ok(c) => c,
        Err(_) => return Vec::new(),
    };
    let value: serde_json::Value = match serde_json::from_str(&content) {
        Ok(v) => v,
        Err(_) => return Vec::new(),
    };
    let Some(projects) = value.get("projects").and_then(|p| p.as_object()) else {
        return Vec::new();
    };
    let mut paths: Vec<String> = projects.keys().map(|k| k.to_string()).collect();
    paths.sort();
    paths
}
