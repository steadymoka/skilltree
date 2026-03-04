use std::path::Path;
use std::process::Command;

use anyhow::{bail, Context, Result};

pub trait GitClient {
    fn shallow_clone(&self, url: &str, git_ref: &str, dest: &Path) -> Result<()>;
    fn sparse_checkout(&self, repo_dir: &Path, path: &str) -> Result<()>;
    fn ls_remote(&self, url: &str, git_ref: &str) -> Result<String>;
}

pub struct RealGitClient;

impl RealGitClient {
    pub fn ensure_git() -> Result<()> {
        let status = Command::new("git")
            .arg("--version")
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status();
        match status {
            Ok(s) if s.success() => Ok(()),
            _ => bail!("git is required but not found. Install git and retry."),
        }
    }
}

impl GitClient for RealGitClient {
    fn shallow_clone(&self, url: &str, git_ref: &str, dest: &Path) -> Result<()> {
        let output = Command::new("git")
            .args([
                "clone",
                "--depth",
                "1",
                "--branch",
                git_ref,
                "--filter=blob:none",
                "--sparse",
                url,
            ])
            .arg(dest)
            .output()
            .context("failed to run git clone")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            bail!("git clone failed: {}", stderr.trim());
        }
        Ok(())
    }

    fn sparse_checkout(&self, repo_dir: &Path, path: &str) -> Result<()> {
        let output = Command::new("git")
            .args(["sparse-checkout", "set", path])
            .current_dir(repo_dir)
            .output()
            .context("failed to run git sparse-checkout")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            bail!("git sparse-checkout failed: {}", stderr.trim());
        }
        Ok(())
    }

    fn ls_remote(&self, url: &str, git_ref: &str) -> Result<String> {
        let output = Command::new("git")
            .args(["ls-remote", url, &format!("refs/heads/{}", git_ref)])
            .output()
            .context("failed to run git ls-remote")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            bail!("git ls-remote failed: {}", stderr.trim());
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let sha = stdout.split_whitespace().next().ok_or_else(|| {
            anyhow::anyhow!("no SHA returned from git ls-remote for ref '{}'", git_ref)
        })?;
        Ok(sha.to_string())
    }
}

#[cfg(test)]
pub mod mock {
    use super::*;
    use std::cell::RefCell;
    use std::fs;

    pub struct MockGitClient {
        pub ls_remote_sha: RefCell<String>,
        pub skills_to_create: Vec<String>,
        pub fail_clone: bool,
    }

    impl MockGitClient {
        pub fn new(sha: &str, skills: Vec<&str>) -> Self {
            Self {
                ls_remote_sha: RefCell::new(sha.to_string()),
                skills_to_create: skills.into_iter().map(String::from).collect(),
                fail_clone: false,
            }
        }

        pub fn failing_clone() -> Self {
            Self {
                ls_remote_sha: RefCell::new(String::new()),
                skills_to_create: vec![],
                fail_clone: true,
            }
        }
    }

    impl GitClient for MockGitClient {
        fn shallow_clone(&self, _url: &str, _git_ref: &str, dest: &Path) -> Result<()> {
            if self.fail_clone {
                anyhow::bail!("mock: clone failed");
            }
            fs::create_dir_all(dest)?;
            // Create .git to simulate real clone
            fs::create_dir_all(dest.join(".git"))?;

            if self.skills_to_create.is_empty() {
                fs::write(
                    dest.join("SKILL.md"),
                    "---\nname: root-skill\ndescription: test skill\n---\n# Root",
                )?;
            } else {
                for name in &self.skills_to_create {
                    let skill_dir = dest.join(name);
                    fs::create_dir_all(&skill_dir)?;
                    fs::write(
                        skill_dir.join("SKILL.md"),
                        format!("---\nname: {name}\ndescription: test {name}\n---\n# {name}"),
                    )?;
                }
            }
            Ok(())
        }

        fn sparse_checkout(&self, repo_dir: &Path, path: &str) -> Result<()> {
            let target = repo_dir.join(path);
            if !target.exists() {
                fs::create_dir_all(&target)?;
                fs::write(
                    target.join("SKILL.md"),
                    format!("---\nname: {path}\ndescription: sparse {path}\n---\n# {path}"),
                )?;
            }
            Ok(())
        }

        fn ls_remote(&self, _url: &str, _git_ref: &str) -> Result<String> {
            Ok(self.ls_remote_sha.borrow().clone())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ensure_git_available() {
        // This test only validates that git exists on the dev machine
        assert!(RealGitClient::ensure_git().is_ok());
    }
}
