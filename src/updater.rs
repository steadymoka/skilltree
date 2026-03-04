use anyhow::{bail, Result};

use crate::adder::{self, AddOpts};
use crate::config::Paths;
use crate::git::GitClient;
use crate::lock::{self, SkillLock};

#[derive(Debug)]
pub struct UpdateResult {
    pub skill_name: String,
    pub old_sha: String,
    pub new_sha: String,
}

pub fn update_skill(
    paths: &Paths,
    skill_name: &str,
    git: &dyn GitClient,
) -> Result<Option<UpdateResult>> {
    let lock_data = lock::read_lock(&paths.skill_lock_json)?;
    update_skill_with_lock(paths, skill_name, &lock_data, git)
}

fn update_skill_with_lock(
    paths: &Paths,
    skill_name: &str,
    lock_data: &SkillLock,
    git: &dyn GitClient,
) -> Result<Option<UpdateResult>> {
    let entry = lock_data
        .get(skill_name)
        .ok_or_else(|| anyhow::anyhow!("skill '{}' not found in lock file", skill_name))?;

    let (owner, repo) = adder::parse_github_source(&entry.source)?;
    let url = adder::github_url(&owner, &repo);
    let remote_sha = git.ls_remote(&url, &entry.git_ref)?;

    if remote_sha == entry.installed_sha {
        return Ok(None);
    }

    let old_sha = entry.installed_sha.clone();
    let skill_path = if entry.skill_path == "." {
        None
    } else {
        Some(entry.skill_path.clone())
    };

    let tags = crate::yaml::read_skills_yaml_or_empty(&paths.skills_yaml)?
        .get(skill_name)
        .cloned()
        .unwrap_or_default();

    let opts = AddOpts {
        source: entry.source.clone(),
        skill: skill_path,
        name: Some(skill_name.to_string()),
        tags,
        force: true,
        git_ref: entry.git_ref.clone(),
    };

    adder::add_skill(paths, &opts, git)?;

    Ok(Some(UpdateResult {
        skill_name: skill_name.to_string(),
        old_sha,
        new_sha: remote_sha,
    }))
}

pub fn update_all(paths: &Paths, git: &dyn GitClient) -> Result<Vec<UpdateResult>> {
    let lock_data = lock::read_lock(&paths.skill_lock_json)?;
    if lock_data.skills.is_empty() {
        bail!("no skills installed. Use 'skilltree add' first.");
    }

    let mut results = Vec::new();

    for name in lock_data.skills.keys() {
        match update_skill_with_lock(paths, name, &lock_data, git) {
            Ok(Some(r)) => results.push(r),
            Ok(None) => {}
            Err(e) => eprintln!("warning: failed to update '{}': {}", name, e),
        }
    }

    Ok(results)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::git::mock::MockGitClient;
    use crate::lock::SkillLockEntry;
    use std::fs;
    use tempfile::TempDir;

    fn setup_with_lock(skill_name: &str, source: &str, installed_sha: &str) -> (TempDir, Paths) {
        let home = TempDir::new().unwrap();
        let paths = Paths::from_home(home.path());
        fs::create_dir_all(&paths.skill_tree_dir).unwrap();

        let skill_dir = paths.skill_tree_dir.join(skill_name);
        fs::create_dir_all(&skill_dir).unwrap();
        fs::write(skill_dir.join("SKILL.md"), "---\nname: test\n---").unwrap();

        let mut lock = lock::SkillLock::new();
        lock.insert(
            skill_name.to_string(),
            SkillLockEntry::new(source, ".", "main", installed_sha),
        );
        lock::write_lock(&paths.skill_lock_json, &lock).unwrap();

        let mut map = crate::yaml::SkillTagMap::new();
        map.insert(skill_name.to_string(), vec!["api".into()]);
        crate::yaml::write_skills_yaml(&paths.skills_yaml, &map).unwrap();

        (home, paths)
    }

    #[test]
    fn update_when_sha_differs() {
        let (_home, paths) = setup_with_lock("my-skill", "user/repo", "old-sha");
        let git = MockGitClient::new("new-sha", vec![]);

        let result = update_skill(&paths, "my-skill", &git).unwrap();
        let r = result.expect("should return Some when SHA differs");
        assert_eq!(r.skill_name, "my-skill");
        assert_eq!(r.old_sha, "old-sha");
        assert_eq!(r.new_sha, "new-sha");

        // Lock updated
        let lock = lock::read_lock(&paths.skill_lock_json).unwrap();
        assert_eq!(lock.get("my-skill").unwrap().installed_sha, "new-sha");
    }

    #[test]
    fn update_preserves_tags() {
        let (_home, paths) = setup_with_lock("my-skill", "user/repo", "old-sha");
        let git = MockGitClient::new("new-sha", vec![]);

        update_skill(&paths, "my-skill", &git).unwrap();

        let map = crate::yaml::read_skills_yaml(&paths.skills_yaml).unwrap();
        assert_eq!(map["my-skill"], vec!["api".to_string()]);
    }

    #[test]
    fn update_already_latest() {
        let (_home, paths) = setup_with_lock("my-skill", "user/repo", "same-sha");
        let git = MockGitClient::new("same-sha", vec![]);

        let result = update_skill(&paths, "my-skill", &git).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn update_not_installed() {
        let home = TempDir::new().unwrap();
        let paths = Paths::from_home(home.path());
        fs::create_dir_all(&paths.skill_tree_dir).unwrap();

        let git = MockGitClient::new("sha", vec![]);
        let err = update_skill(&paths, "nonexistent", &git).unwrap_err();
        assert!(err.to_string().contains("not found in lock file"));
    }

    #[test]
    fn update_with_skill_path() {
        let home = TempDir::new().unwrap();
        let paths = Paths::from_home(home.path());
        fs::create_dir_all(&paths.skill_tree_dir).unwrap();

        let skill_dir = paths.skill_tree_dir.join("auth");
        fs::create_dir_all(&skill_dir).unwrap();
        fs::write(skill_dir.join("SKILL.md"), "---").unwrap();

        let mut lock = lock::SkillLock::new();
        lock.insert(
            "auth".into(),
            SkillLockEntry::new("u/r", "skills/auth", "main", "old-sha"),
        );
        lock::write_lock(&paths.skill_lock_json, &lock).unwrap();

        let mut map = crate::yaml::SkillTagMap::new();
        map.insert("auth".into(), vec![]);
        crate::yaml::write_skills_yaml(&paths.skills_yaml, &map).unwrap();

        let git = MockGitClient::new("new-sha", vec!["skills/auth"]);

        let result = update_skill(&paths, "auth", &git).unwrap();
        let r = result.expect("should update when SHA differs");
        assert_eq!(r.skill_name, "auth");

        // Lock should record the skill_path
        let lock = lock::read_lock(&paths.skill_lock_json).unwrap();
        assert_eq!(lock.get("auth").unwrap().skill_path, "skills/auth");
    }

    #[test]
    fn update_all_mixed_updated_and_latest() {
        let home = TempDir::new().unwrap();
        let paths = Paths::from_home(home.path());
        fs::create_dir_all(&paths.skill_tree_dir).unwrap();

        for name in ["skill-a", "skill-b"] {
            let dir = paths.skill_tree_dir.join(name);
            fs::create_dir_all(&dir).unwrap();
            fs::write(dir.join("SKILL.md"), "---").unwrap();
        }

        let mut lock = lock::SkillLock::new();
        lock.insert(
            "skill-a".into(),
            SkillLockEntry::new("u/a", ".", "main", "old-sha"),
        );
        // skill-b already at the SHA that mock will return
        lock.insert(
            "skill-b".into(),
            SkillLockEntry::new("u/b", ".", "main", "current-sha"),
        );
        lock::write_lock(&paths.skill_lock_json, &lock).unwrap();

        let mut map = crate::yaml::SkillTagMap::new();
        map.insert("skill-a".into(), vec![]);
        map.insert("skill-b".into(), vec![]);
        crate::yaml::write_skills_yaml(&paths.skills_yaml, &map).unwrap();

        // Mock returns "current-sha" → skill-a updates (old-sha != current-sha), skill-b stays
        let git = MockGitClient::new("current-sha", vec![]);

        let results = update_all(&paths, &git).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].skill_name, "skill-a");
        assert_eq!(results[0].old_sha, "old-sha");
        assert_eq!(results[0].new_sha, "current-sha");
    }

    #[test]
    fn update_all_empty_lock() {
        let home = TempDir::new().unwrap();
        let paths = Paths::from_home(home.path());
        fs::create_dir_all(&paths.skill_tree_dir).unwrap();

        let git = MockGitClient::new("sha", vec![]);
        let err = update_all(&paths, &git).unwrap_err();
        assert!(err.to_string().contains("no skills installed"));
    }
}
