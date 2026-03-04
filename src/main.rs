use std::env;
use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};

use skilltree::config::Paths;
use skilltree::fs_util::Tool;
use skilltree::git::RealGitClient;
use skilltree::http::UreqHttpClient;
use skilltree::{adder, finder, init, linker, serve, tagger, tree, tui, updater};

#[derive(Parser)]
#[command(
    name = "skilltree",
    about = "Skill management tool for Claude Code & Codex"
)]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand)]
enum Command {
    /// Initialize skilltree (idempotent)
    Init,

    /// Link skills to the current project by tags
    Link {
        /// Tags to match (union)
        #[arg(required = true)]
        tags: Vec<String>,

        /// Project path (defaults to CWD)
        #[arg(long, short)]
        path: Option<PathBuf>,

        /// Target tool (claude or codex)
        #[arg(long, short, default_value = "claude")]
        tool: String,
    },

    /// Link a single skill by name
    LinkSkill {
        /// Skill directory name
        skill: String,

        /// Project path (defaults to CWD)
        #[arg(long, short)]
        path: Option<PathBuf>,

        /// Target tool (claude or codex)
        #[arg(long, short, default_value = "claude")]
        tool: String,
    },

    /// Unlink a skill from the current project
    Unlink {
        /// Skill directory name to unlink
        skill: Option<String>,

        /// Unlink all skills
        #[arg(long)]
        all: bool,

        /// Project path (defaults to CWD)
        #[arg(long, short)]
        path: Option<PathBuf>,

        /// Target tool (claude or codex)
        #[arg(long, short, default_value = "claude")]
        tool: String,
    },

    /// Set tags for a skill (used internally by web UI)
    #[command(hide = true)]
    Tag { skill: String, tags: Vec<String> },

    /// Add a skill from GitHub
    Add {
        /// GitHub source (owner/repo)
        source: String,

        /// Specific skill path within repo
        #[arg(long)]
        skill: Option<String>,

        /// Custom local name
        #[arg(long)]
        name: Option<String>,

        /// Tags to assign
        #[arg(long, short)]
        tag: Vec<String>,

        /// Overwrite existing
        #[arg(long)]
        force: bool,

        /// Git ref (branch/tag)
        #[arg(long, default_value = "main")]
        git_ref: String,
    },

    /// Search for skills on GitHub
    Find {
        /// Search query
        query: String,

        /// Max results
        #[arg(long, short, default_value = "10")]
        limit: usize,
    },

    /// Update installed skill(s) to latest
    Update {
        /// Skill name (omit for all)
        skill: Option<String>,
    },

    /// Print skill tree grouped by tags
    Tree,

    /// Start the web UI (Next.js standalone server)
    Serve {
        /// Project root containing .next/standalone/
        #[arg(long)]
        root: Option<PathBuf>,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    let Some(command) = cli.command else {
        let paths = Paths::default_paths()?;
        let project_paths = load_project_paths();
        return tui::run(paths, project_paths);
    };

    match command {
        Command::Serve { root } => serve::start_web(root),

        _ => {
            let paths = Paths::default_paths()?;
            dispatch(command, &paths)
        }
    }
}

fn dispatch(command: Command, paths: &Paths) -> Result<()> {
    match command {
        Command::Init => init::initialize(paths),

        Command::Add {
            source,
            skill,
            name,
            tag,
            force,
            git_ref,
        } => {
            RealGitClient::ensure_git()?;
            let opts = adder::AddOpts {
                source,
                skill,
                name,
                tags: tag,
                force,
                git_ref,
            };
            let git = RealGitClient;
            let result = adder::add_skill(paths, &opts, &git)?;
            println!("Added: {}", result.skill_name);
            Ok(())
        }

        Command::Find { query, limit } => {
            let http = UreqHttpClient;
            let opts = finder::FindOpts { query, limit };
            let results = finder::find_skills(&opts, &http)?;
            if results.is_empty() {
                println!("No skills found.");
            } else {
                for s in &results {
                    println!("{} ({} stars) - {}", s.full_name, s.stars, s.description);
                }
            }
            Ok(())
        }

        Command::Update { skill } => {
            RealGitClient::ensure_git()?;
            let git = RealGitClient;
            if let Some(name) = skill {
                match updater::update_skill(paths, &name, &git)? {
                    Some(r) => print_update(&r),
                    None => println!("'{}' is already up to date.", name),
                }
            } else {
                let results = updater::update_all(paths, &git)?;
                if results.is_empty() {
                    println!("All skills are up to date.");
                } else {
                    for r in &results {
                        print_update(r);
                    }
                    println!("{} skill(s) updated.", results.len());
                }
            }
            Ok(())
        }

        Command::Link { tags, path, tool } => {
            let project = resolve_project(path)?;
            let tool: Tool = tool.parse()?;
            let linked = linker::link_by_tags(paths, &project, &tags, tool)?;
            println!("Done. {} skill(s) linked.", linked);
            Ok(())
        }

        Command::LinkSkill { skill, path, tool } => {
            let project = resolve_project(path)?;
            let tool: Tool = tool.parse()?;
            linker::link_skill(paths, &project, &skill, tool)?;
            println!("Linked: {}", skill);
            Ok(())
        }

        Command::Unlink {
            skill,
            all,
            path,
            tool,
        } => {
            let project = resolve_project(path)?;
            let tool: Tool = tool.parse()?;
            if all {
                let removed = linker::unlink_all(&project, tool)?;
                println!("Done. {} skill(s) unlinked.", removed);
                Ok(())
            } else if let Some(name) = skill {
                linker::unlink_skill(&project, &name, tool)?;
                println!("Unlinked: {}", name);
                Ok(())
            } else {
                anyhow::bail!("specify a skill name or --all");
            }
        }

        Command::Tag { skill, tags } => {
            tagger::set_tags(paths, &skill, &tags)?;
            println!("{}: [{}]", skill, tags.join(", "));
            Ok(())
        }

        Command::Tree => {
            let project_paths = load_project_paths();
            tree::print_tree(paths, &project_paths)
        }

        Command::Serve { .. } => unreachable!(),
    }
}

fn print_update(r: &updater::UpdateResult) {
    let end = |s: &str| 7.min(s.len());
    println!(
        "Updated '{}': {} → {}",
        r.skill_name,
        &r.old_sha[..end(&r.old_sha)],
        &r.new_sha[..end(&r.new_sha)],
    );
}

fn resolve_project(path: Option<PathBuf>) -> Result<PathBuf> {
    match path {
        Some(p) => Ok(p),
        None => env::current_dir().context("cannot determine current directory"),
    }
}

/// Load project paths from ~/.claude.json for TUI project list.
fn load_project_paths() -> Vec<String> {
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
