use std::collections::{BTreeMap, BTreeSet, HashSet};
use std::io;
use std::path::Path;

use anyhow::Result;
use crossterm::event::{self, Event};
use crossterm::execute;
use crossterm::terminal::{self, EnterAlternateScreen, LeaveAlternateScreen};
use ratatui::prelude::*;
use ratatui::widgets::ListState;

use crate::config::Paths;
use crate::fs_util::Tool;
use crate::scanner;
use crate::yaml::{self, SkillTagMap};

use super::ui;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Screen {
    Skills,
    Claude,
    Codex,
}

impl Screen {
    pub(super) fn tool(self) -> Tool {
        match self {
            Screen::Claude => Tool::Claude,
            Screen::Codex => Tool::Codex,
            Screen::Skills => unreachable!("tool() called on Skills screen"),
        }
    }

    pub(super) fn is_projects(self) -> bool {
        matches!(self, Screen::Claude | Screen::Codex)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Panel {
    Left,
    Middle,
    Right,
}

pub(super) struct TextInputState {
    pub input: String,
    pub cursor: usize,
}

pub(super) struct SkillsScreenState {
    pub selected_skill: usize,
    pub selected_tag: usize,
    pub selected_project_link: usize,
    pub skill_list_state: ListState,
    pub tag_list_state: ListState,
    pub project_link_list_state: ListState,
}

#[derive(Debug, Clone)]
pub(super) struct SkillProjectLink {
    pub project_path: String,
    pub project_name: String,
    pub tool: Tool,
}

#[derive(Debug, Clone)]
pub(super) struct SkillProjectEntry {
    pub project_path: String,
    pub project_name: String,
    pub has_claude: bool,
    pub has_codex: bool,
}

pub(super) struct UnlinkModalState {
    pub skill: String,
    pub project_path: String,
    pub project_name: String,
    pub claude_linked: bool,
    pub codex_linked: bool,
    pub claude_checked: bool,
    pub codex_checked: bool,
    pub cursor: usize,
}

pub(super) struct DeleteModalState {
    pub skill: String,
}

pub(super) struct ProjectsScreenState {
    pub selected_project: usize,
    pub tree_cursor: usize,
    pub collapsed_tags: HashSet<String>,
    pub tree_list_state: ListState,
    pub project_list_state: ListState,
}

#[derive(Debug, Clone)]
pub(super) enum TreeRow {
    TagHeader { tag: String, skill_count: usize },
    Skill { skill: String },
    UntaggedHeader { skill_count: usize },
    UntaggedSkill { skill: String },
}

pub struct App {
    pub(super) paths: Paths,
    pub(super) tag_map: SkillTagMap,
    pub(super) all_tags: Vec<String>,
    pub(super) skill_dirs: Vec<String>,
    pub(super) skill_dir_set: HashSet<String>,
    pub(super) project_paths: Vec<String>,
    pub(super) project_links: BTreeMap<String, HashSet<String>>,

    pub(super) screen: Screen,
    pub(super) panel: Panel,
    pub(super) status_msg: String,
    pub(super) should_quit: bool,

    pub(super) skills_state: SkillsScreenState,
    pub(super) projects_state: ProjectsScreenState,

    pub(super) text_input: Option<TextInputState>,
    pub(super) tree_rows: Vec<TreeRow>,
    pub(super) tag_skills: BTreeMap<String, Vec<String>>,
    pub(super) skill_project_links: BTreeMap<String, Vec<SkillProjectLink>>,
    pub(super) unlink_modal: Option<UnlinkModalState>,
    pub(super) delete_modal: Option<DeleteModalState>,
}

impl App {
    pub fn new(paths: Paths, project_paths: Vec<String>) -> Result<Self> {
        let mut all_paths = vec![paths.home_dir.to_string_lossy().to_string()];
        all_paths.extend(project_paths);
        let mut app = App {
            paths,
            tag_map: BTreeMap::new(),
            all_tags: Vec::new(),
            skill_dirs: Vec::new(),
            skill_dir_set: HashSet::new(),
            project_paths: all_paths,
            project_links: BTreeMap::new(),

            screen: Screen::Skills,
            panel: Panel::Left,
            status_msg: String::new(),
            should_quit: false,

            skills_state: SkillsScreenState {
                selected_skill: 0,
                selected_tag: 0,
                selected_project_link: 0,
                skill_list_state: ListState::default(),
                tag_list_state: ListState::default(),
                project_link_list_state: ListState::default(),
            },
            projects_state: ProjectsScreenState {
                selected_project: 0,
                tree_cursor: 0,
                collapsed_tags: HashSet::new(),
                tree_list_state: ListState::default(),
                project_list_state: ListState::default(),
            },

            text_input: None,
            tree_rows: Vec::new(),
            tag_skills: BTreeMap::new(),
            skill_project_links: BTreeMap::new(),
            unlink_modal: None,
            delete_modal: None,
        };
        app.reload()?;
        Ok(app)
    }

    pub(super) fn reload(&mut self) -> Result<()> {
        self.tag_map = yaml::read_skills_yaml_or_empty(&self.paths.skills_yaml)?;
        self.skill_dirs = scanner::scan_skill_dirs(&self.paths.skill_tree_dir)?;
        self.skill_dir_set = self.skill_dirs.iter().cloned().collect();

        let mut tags = BTreeSet::new();
        for t in self.tag_map.values().flatten() {
            tags.insert(t.clone());
        }
        self.all_tags = tags.into_iter().collect();

        self.reload_all_project_links_all_tools();
        if self.screen.is_projects() {
            self.reload_all_project_links();
        }
        self.rebuild_tree();
        self.sync_list_states();
        Ok(())
    }

    pub(super) fn reload_project_links(&mut self, project: &str) {
        let linked = scanner::scan_linked_skills(std::path::Path::new(project), self.screen.tool());
        self.project_links
            .insert(project.to_string(), linked.into_iter().collect());
    }

    pub(super) fn reload_all_project_links(&mut self) {
        self.project_links.clear();
        let paths = self.project_paths.clone();
        for p in &paths {
            self.reload_project_links(p);
        }
    }

    pub(super) fn rebuild_tree(&mut self) {
        let mut rows = Vec::new();
        let collapsed = &self.projects_state.collapsed_tags;

        let mut by_tag: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
        let mut tagged_skills: HashSet<&str> = HashSet::new();

        for (skill, tags) in &self.tag_map {
            if !self.skill_dir_set.contains(skill.as_str()) {
                continue;
            }
            for tag in tags {
                by_tag.entry(tag.as_str()).or_default().push(skill.as_str());
                tagged_skills.insert(skill.as_str());
            }
        }

        let mut untagged: Vec<&str> = Vec::new();
        for skill in &self.skill_dirs {
            if !tagged_skills.contains(skill.as_str()) {
                untagged.push(skill.as_str());
            }
        }

        for (tag, skills) in &by_tag {
            rows.push(TreeRow::TagHeader {
                tag: tag.to_string(),
                skill_count: skills.len(),
            });
            if !collapsed.contains(*tag) {
                for skill in skills {
                    rows.push(TreeRow::Skill {
                        skill: skill.to_string(),
                    });
                }
            }
        }

        if !untagged.is_empty() {
            rows.push(TreeRow::UntaggedHeader {
                skill_count: untagged.len(),
            });
            if !collapsed.contains("__untagged__") {
                for skill in &untagged {
                    rows.push(TreeRow::UntaggedSkill {
                        skill: skill.to_string(),
                    });
                }
            }
        }

        self.tree_rows = rows;
        self.tag_skills = by_tag
            .into_iter()
            .map(|(tag, skills)| {
                (
                    tag.to_string(),
                    skills.into_iter().map(|s| s.to_string()).collect(),
                )
            })
            .collect();
    }

    pub(super) fn reload_all_project_links_all_tools(&mut self) {
        let mut map: BTreeMap<String, Vec<SkillProjectLink>> = BTreeMap::new();
        for p in &self.project_paths.clone() {
            let name = self.display_project_name(p);
            for tool in [Tool::Claude, Tool::Codex] {
                for skill in scanner::scan_linked_skills(Path::new(p), tool) {
                    map.entry(skill).or_default().push(SkillProjectLink {
                        project_path: p.clone(),
                        project_name: name.clone(),
                        tool,
                    });
                }
            }
        }
        self.skill_project_links = map;
    }

    pub(super) fn skill_count(&self) -> usize {
        self.skill_dirs.len()
    }

    pub(super) fn tags_for_skill(&self, skill: &str) -> &[String] {
        self.tag_map.get(skill).map(|v| v.as_slice()).unwrap_or(&[])
    }

    pub(super) fn selected_project_path(&self) -> Option<&str> {
        self.project_paths
            .get(self.projects_state.selected_project)
            .map(|s| s.as_str())
    }

    pub(super) fn is_skill_linked_to_selected(&self, skill: &str) -> bool {
        self.selected_project_path()
            .and_then(|p| self.project_links.get(p))
            .is_some_and(|set| set.contains(skill))
    }

    pub(super) fn linked_count(&self, project: &str) -> usize {
        self.project_links.get(project).map_or(0, |s| s.len())
    }

    pub(super) fn skills_for_tag(&self, tag: &str) -> &[String] {
        self.tag_skills
            .get(tag)
            .map(|v| v.as_slice())
            .unwrap_or(&[])
    }

    pub(super) fn selected_skill_project_entries(&self) -> Vec<SkillProjectEntry> {
        let skill = match self.skill_dirs.get(self.skills_state.selected_skill) {
            Some(s) => s,
            None => return Vec::new(),
        };
        let links = match self.skill_project_links.get(skill) {
            Some(l) => l,
            None => return Vec::new(),
        };
        let mut by_project: BTreeMap<&str, (String, bool, bool)> = BTreeMap::new();
        for link in links {
            let entry = by_project
                .entry(&link.project_name)
                .or_insert_with(|| (link.project_path.clone(), false, false));
            match link.tool {
                Tool::Claude => entry.1 = true,
                Tool::Codex => entry.2 = true,
                _ => {}
            }
        }
        by_project
            .into_values()
            .map(|(path, claude, codex)| SkillProjectEntry {
                project_name: self.display_project_name(&path),
                project_path: path,
                has_claude: claude,
                has_codex: codex,
            })
            .collect()
    }

    pub(super) fn skill_linked_project_count(&self, skill: &str) -> usize {
        self.skill_project_links
            .get(skill)
            .map(|links| {
                let mut projects = HashSet::new();
                for l in links {
                    projects.insert(&l.project_path);
                }
                projects.len()
            })
            .unwrap_or(0)
    }

    pub(super) fn display_project_name(&self, path: &str) -> String {
        if path == self.paths.home_dir.to_string_lossy() {
            "Global".to_string()
        } else {
            crate::fs_util::basename(path).to_string()
        }
    }

    pub(super) fn clamp_all_selections(&mut self) {
        let skill_len = self.skill_dirs.len();
        if skill_len > 0 && self.skills_state.selected_skill >= skill_len {
            self.skills_state.selected_skill = skill_len - 1;
        }

        let tag_len = self.all_tags.len();
        if tag_len > 0 && self.skills_state.selected_tag >= tag_len {
            self.skills_state.selected_tag = tag_len - 1;
        }

        let proj_len = self.project_paths.len();
        if proj_len > 0 && self.projects_state.selected_project >= proj_len {
            self.projects_state.selected_project = proj_len - 1;
        }

        let tree_len = self.tree_rows.len();
        if tree_len > 0 && self.projects_state.tree_cursor >= tree_len {
            self.projects_state.tree_cursor = tree_len - 1;
        }

        let proj_link_len = self.selected_skill_project_entries().len();
        if proj_link_len > 0 && self.skills_state.selected_project_link >= proj_link_len {
            self.skills_state.selected_project_link = proj_link_len - 1;
        }

        self.sync_list_states();
    }

    pub(super) fn sync_list_states(&mut self) {
        self.skills_state
            .skill_list_state
            .select(Some(self.skills_state.selected_skill));
        if !self.all_tags.is_empty() {
            self.skills_state
                .tag_list_state
                .select(Some(self.skills_state.selected_tag));
        }
        let entries = self.selected_skill_project_entries();
        if !entries.is_empty() {
            self.skills_state
                .project_link_list_state
                .select(Some(self.skills_state.selected_project_link));
        }
        self.projects_state
            .tree_list_state
            .select(Some(self.projects_state.tree_cursor));
        if !self.project_paths.is_empty() {
            self.projects_state
                .project_list_state
                .select(Some(self.projects_state.selected_project));
        }
    }
}

struct TerminalGuard;

impl TerminalGuard {
    fn new() -> Result<Self> {
        terminal::enable_raw_mode()?;
        execute!(io::stdout(), EnterAlternateScreen)?;
        Ok(Self)
    }
}

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        let _ = terminal::disable_raw_mode();
        let _ = execute!(io::stdout(), LeaveAlternateScreen);
    }
}

#[cfg(test)]
impl App {
    pub(super) fn new_test() -> Self {
        App {
            paths: Paths::from_home(std::path::Path::new("/tmp/test")),
            tag_map: BTreeMap::new(),
            all_tags: Vec::new(),
            skill_dirs: Vec::new(),
            skill_dir_set: HashSet::new(),
            project_paths: vec!["proj1".into()],
            project_links: BTreeMap::new(),
            screen: Screen::Skills,
            panel: Panel::Left,
            status_msg: String::new(),
            should_quit: false,
            skills_state: SkillsScreenState {
                selected_skill: 0,
                selected_tag: 0,
                selected_project_link: 0,
                skill_list_state: ListState::default(),
                tag_list_state: ListState::default(),
                project_link_list_state: ListState::default(),
            },
            projects_state: ProjectsScreenState {
                selected_project: 0,
                tree_cursor: 0,
                collapsed_tags: HashSet::new(),
                tree_list_state: ListState::default(),
                project_list_state: ListState::default(),
            },
            text_input: None,
            tree_rows: Vec::new(),
            tag_skills: BTreeMap::new(),
            skill_project_links: BTreeMap::new(),
            unlink_modal: None,
            delete_modal: None,
        }
    }
}

pub fn run(paths: Paths, project_paths: Vec<String>) -> Result<()> {
    let _guard = TerminalGuard::new()?;
    let backend = CrosstermBackend::new(io::stdout());
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::new(paths, project_paths)?;

    loop {
        terminal.draw(|frame| ui::render(frame, &mut app))?;

        if let Event::Key(key) = event::read()? {
            if key.kind == event::KeyEventKind::Press {
                app.handle_key(key.code, key.modifiers)?;
            }
        }

        if app.should_quit {
            break;
        }
    }

    Ok(())
}
