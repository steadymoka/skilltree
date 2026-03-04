use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

use anyhow::Result;

use crate::config::Paths;
use crate::fs_util;
use crate::lock;
use crate::scanner;
use crate::yaml;

pub fn run(paths: &Paths, fix: bool, project_paths: &[String]) -> Result<()> {
    let mut issues = 0;

    println!("Checking {} ...", paths.skill_tree_dir.display());

    // 1. skills.yaml validity
    let mut yaml_map = match yaml::read_skills_yaml(&paths.skills_yaml) {
        Ok(map) => {
            println!("  \u{2713} skills.yaml is valid ({} skills)", map.len());
            map
        }
        Err(e) => {
            println!("  ! skills.yaml is broken: {}", e);
            issues += 1;
            return finish(issues);
        }
    };

    let yaml_names: BTreeSet<String> = yaml_map.keys().cloned().collect();
    let dir_names: BTreeSet<String> = scanner::scan_real_dirs(&paths.skill_tree_dir)?
        .into_iter()
        .collect();

    // Ghost entries: in yaml but no directory
    let ghosts: Vec<&String> = yaml_names.difference(&dir_names).collect();
    if ghosts.is_empty() {
        println!("  \u{2713} All yaml entries have matching directories");
    } else {
        for name in &ghosts {
            println!(
                "  ! Ghost entry in skills.yaml: {} (directory missing)",
                name
            );
            issues += 1;
        }
        if fix {
            for name in &ghosts {
                yaml_map.remove(name.as_str());
                println!("    Fixed: removed '{}' from skills.yaml", name);
            }
            yaml::write_skills_yaml(&paths.skills_yaml, &yaml_map)?;
        }
    }

    // Unregistered directories: on disk but not in yaml
    let unregistered: Vec<&String> = dir_names.difference(&yaml_names).collect();
    if unregistered.is_empty() {
        println!("  \u{2713} All skill directories are registered in skills.yaml");
    } else {
        for name in &unregistered {
            println!("  ! Unregistered directory: {} (not in skills.yaml)", name);
            issues += 1;
        }
        if fix {
            for name in &unregistered {
                yaml_map.insert(name.to_string(), Vec::new());
                println!("    Fixed: added '{}' to skills.yaml", name);
            }
            yaml::write_skills_yaml(&paths.skills_yaml, &yaml_map)?;
        }
    }

    // 3. Lock file consistency
    let mut lock_data = lock::read_lock(&paths.skill_lock_json)?;
    let lock_orphans: Vec<String> = lock_data
        .skills
        .keys()
        .filter(|name| !dir_names.contains(name.as_str()))
        .cloned()
        .collect();

    if lock_orphans.is_empty() {
        println!("  \u{2713} .skill-lock.json is consistent");
    } else {
        for name in &lock_orphans {
            println!(
                "  ! Orphan in .skill-lock.json: {} (directory missing)",
                name
            );
            issues += 1;
        }
        if fix {
            for name in &lock_orphans {
                lock_data.remove(name.as_str());
                println!("    Fixed: removed '{}' from .skill-lock.json", name);
            }
            lock::write_lock(&paths.skill_lock_json, &lock_data)?;
        }
    }

    // 4. Broken symlinks in projects
    let mut found_broken = false;
    for project in project_paths {
        let project_path = Path::new(project);
        for tool in fs_util::ALL_TOOLS {
            let skills_dir = fs_util::project_skills_dir(project_path, tool);
            let entries = match fs::read_dir(&skills_dir) {
                Ok(e) => e,
                Err(_) => continue,
            };
            for entry in entries.flatten() {
                let meta = match fs::symlink_metadata(entry.path()) {
                    Ok(m) => m,
                    Err(_) => continue,
                };
                if !meta.file_type().is_symlink() {
                    continue;
                }
                if !entry.path().exists() {
                    let target = fs::read_link(entry.path()).unwrap_or_default();
                    println!(
                        "  ! Broken symlink: {} -> {} (target missing)",
                        entry.path().display(),
                        target.display()
                    );
                    issues += 1;
                    found_broken = true;
                    if fix {
                        fs::remove_file(entry.path())?;
                        println!("    Fixed: removed broken symlink");
                    }
                }
            }
        }
    }
    if !found_broken && !project_paths.is_empty() {
        println!(
            "  \u{2713} No broken symlinks across {} project(s)",
            project_paths.len()
        );
    }

    finish(issues)
}

fn finish(issues: usize) -> Result<()> {
    println!();
    if issues == 0 {
        println!("No issues found.");
    } else {
        println!("{} issue(s) found.", issues);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Paths;
    use tempfile::TempDir;

    fn setup_paths() -> (TempDir, Paths) {
        let home = TempDir::new().unwrap();
        let paths = Paths::from_home(home.path());
        fs::create_dir_all(&paths.skill_tree_dir).unwrap();
        (home, paths)
    }

    #[test]
    fn doctor_clean_state() {
        let (_home, paths) = setup_paths();

        // Create skill dir and yaml
        fs::create_dir(paths.skill_tree_dir.join("my-skill")).unwrap();
        let mut map = yaml::SkillTagMap::new();
        map.insert("my-skill".into(), vec!["tag1".into()]);
        yaml::write_skills_yaml(&paths.skills_yaml, &map).unwrap();

        let result = run(&paths, false, &[]);
        assert!(result.is_ok());
    }

    #[test]
    fn doctor_detects_ghost_entry() {
        let (_home, paths) = setup_paths();

        // yaml has entry but no directory
        let mut map = yaml::SkillTagMap::new();
        map.insert("ghost-skill".into(), vec![]);
        yaml::write_skills_yaml(&paths.skills_yaml, &map).unwrap();

        let result = run(&paths, false, &[]);
        assert!(result.is_ok());
    }

    #[test]
    fn doctor_fix_ghost_entry() {
        let (_home, paths) = setup_paths();

        let mut map = yaml::SkillTagMap::new();
        map.insert("ghost-skill".into(), vec![]);
        yaml::write_skills_yaml(&paths.skills_yaml, &map).unwrap();

        run(&paths, true, &[]).unwrap();

        let updated = yaml::read_skills_yaml(&paths.skills_yaml).unwrap();
        assert!(!updated.contains_key("ghost-skill"));
    }

    #[test]
    fn doctor_fix_unregistered_dir() {
        let (_home, paths) = setup_paths();

        fs::create_dir(paths.skill_tree_dir.join("orphan-skill")).unwrap();
        let map = yaml::SkillTagMap::new();
        yaml::write_skills_yaml(&paths.skills_yaml, &map).unwrap();

        run(&paths, true, &[]).unwrap();

        let updated = yaml::read_skills_yaml(&paths.skills_yaml).unwrap();
        assert!(updated.contains_key("orphan-skill"));
    }

    #[test]
    #[cfg(unix)]
    fn doctor_detects_broken_symlink() {
        use std::os::unix::fs::symlink;

        let (_home, paths) = setup_paths();
        let map = yaml::SkillTagMap::new();
        yaml::write_skills_yaml(&paths.skills_yaml, &map).unwrap();

        // Create a fake project with a broken symlink
        let project = TempDir::new().unwrap();
        let skills_dir = project.path().join(".claude").join("skills");
        fs::create_dir_all(&skills_dir).unwrap();
        symlink("/nonexistent/target", skills_dir.join("broken-skill")).unwrap();

        let project_paths = vec![project.path().to_string_lossy().into_owned()];
        run(&paths, false, &project_paths).unwrap();

        // Broken symlink should still exist (no --fix)
        assert!(skills_dir.join("broken-skill").symlink_metadata().is_ok());
    }

    #[test]
    #[cfg(unix)]
    fn doctor_fix_broken_symlink() {
        use std::os::unix::fs::symlink;

        let (_home, paths) = setup_paths();
        let map = yaml::SkillTagMap::new();
        yaml::write_skills_yaml(&paths.skills_yaml, &map).unwrap();

        let project = TempDir::new().unwrap();
        let skills_dir = project.path().join(".claude").join("skills");
        fs::create_dir_all(&skills_dir).unwrap();
        symlink("/nonexistent/target", skills_dir.join("broken-skill")).unwrap();

        let project_paths = vec![project.path().to_string_lossy().into_owned()];
        run(&paths, true, &project_paths).unwrap();

        // Broken symlink should be removed
        assert!(!skills_dir.join("broken-skill").symlink_metadata().is_ok());
    }
}
