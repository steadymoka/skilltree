use anyhow::Result;
use crossterm::event::{KeyCode, KeyModifiers};

use crate::linker;
use crate::remover;
use crate::tagger;

use super::app::{App, DeleteModalState, Panel, Screen, TextInputState, TreeRow, UnlinkModalState};

impl App {
    pub(super) fn handle_key(&mut self, code: KeyCode, modifiers: KeyModifiers) -> Result<()> {
        if self.text_input.is_some() {
            return self.handle_text_input_key(code);
        }
        if self.unlink_modal.is_some() {
            return self.handle_unlink_modal_key(code);
        }
        if self.delete_modal.is_some() {
            return self.handle_delete_modal_key(code);
        }

        self.status_msg.clear();

        match code {
            KeyCode::Char('c') if modifiers.contains(KeyModifiers::CONTROL) => {
                self.should_quit = true;
            }
            KeyCode::Char('q') | KeyCode::Esc => self.should_quit = true,
            KeyCode::Char('1') => {
                self.screen = Screen::Skills;
                self.panel = Panel::Left;
            }
            KeyCode::Char('2') => self.switch_projects_screen(Screen::Claude),
            KeyCode::Char('3') => self.switch_projects_screen(Screen::Codex),
            KeyCode::Right => {
                self.panel = match (self.screen, self.panel) {
                    (Screen::Skills, Panel::Left) => Panel::Middle,
                    (Screen::Skills, Panel::Middle) => Panel::Right,
                    (_, Panel::Left | Panel::Middle) => Panel::Right,
                    _ => self.panel,
                };
            }
            KeyCode::Left => {
                self.panel = match (self.screen, self.panel) {
                    (Screen::Skills, Panel::Right) => Panel::Middle,
                    (Screen::Skills, Panel::Middle) => Panel::Left,
                    (_, Panel::Right) => Panel::Left,
                    _ => self.panel,
                };
            }
            _ => {
                if self.screen.is_projects() {
                    self.handle_projects_screen_key(code)?;
                } else {
                    self.handle_skills_screen_key(code)?;
                }
            }
        }

        self.clamp_all_selections();
        Ok(())
    }

    fn switch_projects_screen(&mut self, screen: Screen) {
        debug_assert!(screen.is_projects());
        if self.screen == screen {
            return;
        }
        self.screen = screen;
        self.panel = Panel::Left;
        self.reload_all_project_links();
        self.rebuild_tree();
    }

    // ── Screen 1: Skills & Tags ──

    fn handle_skills_screen_key(&mut self, code: KeyCode) -> Result<()> {
        match self.panel {
            Panel::Left => self.handle_skill_list_key(code),
            Panel::Middle => self.handle_tag_list_key(code),
            Panel::Right => self.handle_skill_project_link_key(code),
        }
    }

    fn handle_skill_list_key(&mut self, code: KeyCode) -> Result<()> {
        let len = self.skill_dirs.len();
        let prev = self.skills_state.selected_skill;
        match code {
            KeyCode::Up | KeyCode::Char('k') => {
                self.skills_state.selected_skill =
                    self.skills_state.selected_skill.saturating_sub(1);
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if self.skills_state.selected_skill + 1 < len {
                    self.skills_state.selected_skill += 1;
                }
            }
            KeyCode::Char('d') => {
                if let Some(skill) = self
                    .skill_dirs
                    .get(self.skills_state.selected_skill)
                    .cloned()
                {
                    self.delete_modal = Some(DeleteModalState { skill });
                }
            }
            _ => {}
        }
        if self.skills_state.selected_skill != prev {
            self.skills_state.selected_project_link = 0;
        }
        Ok(())
    }

    fn handle_tag_list_key(&mut self, code: KeyCode) -> Result<()> {
        let len = self.all_tags.len();
        match code {
            KeyCode::Up | KeyCode::Char('k') => {
                self.skills_state.selected_tag = self.skills_state.selected_tag.saturating_sub(1);
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if len > 0 && self.skills_state.selected_tag + 1 < len {
                    self.skills_state.selected_tag += 1;
                }
            }
            KeyCode::Char(' ') | KeyCode::Enter => {
                self.toggle_tag_on_selected_skill()?;
            }
            KeyCode::Char('a') => {
                self.text_input = Some(TextInputState {
                    input: String::new(),
                    cursor: 0,
                });
            }
            _ => {}
        }
        Ok(())
    }

    fn toggle_tag_on_selected_skill(&mut self) -> Result<()> {
        let Some(skill) = self
            .skill_dirs
            .get(self.skills_state.selected_skill)
            .cloned()
        else {
            return Ok(());
        };
        let Some(tag) = self.all_tags.get(self.skills_state.selected_tag).cloned() else {
            return Ok(());
        };

        let has_tag = self
            .tag_map
            .get(&skill)
            .is_some_and(|tags| tags.contains(&tag));

        if has_tag {
            tagger::remove_tag(&self.paths, &skill, &tag)?;
            self.status_msg = format!("{}: removed [{}]", skill, tag);
        } else {
            tagger::add_tag(&self.paths, &skill, &tag)?;
            self.status_msg = format!("{}: added [{}]", skill, tag);
        }
        self.reload()?;
        Ok(())
    }

    fn handle_skill_project_link_key(&mut self, code: KeyCode) -> Result<()> {
        let entries = self.selected_skill_project_entries();
        let len = entries.len();
        match code {
            KeyCode::Up | KeyCode::Char('k') => {
                self.skills_state.selected_project_link =
                    self.skills_state.selected_project_link.saturating_sub(1);
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if len > 0 && self.skills_state.selected_project_link + 1 < len {
                    self.skills_state.selected_project_link += 1;
                }
            }
            KeyCode::Char(' ') | KeyCode::Enter => {
                if let Some(entry) = entries.get(self.skills_state.selected_project_link) {
                    let skill = self
                        .skill_dirs
                        .get(self.skills_state.selected_skill)
                        .cloned()
                        .unwrap_or_default();
                    self.unlink_modal = Some(UnlinkModalState {
                        skill,
                        project_path: entry.project_path.clone(),
                        project_name: entry.project_name.clone(),
                        claude_linked: entry.has_claude,
                        codex_linked: entry.has_codex,
                        claude_checked: entry.has_claude,
                        codex_checked: entry.has_codex,
                        cursor: 0,
                    });
                }
            }
            _ => {}
        }
        Ok(())
    }

    // ── Unlink Modal ──

    fn handle_unlink_modal_key(&mut self, code: KeyCode) -> Result<()> {
        let state = self.unlink_modal.as_mut().unwrap();
        match code {
            KeyCode::Up | KeyCode::Char('k') => {
                state.cursor = state.cursor.saturating_sub(1);
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if state.cursor < 1 {
                    state.cursor = 1;
                }
            }
            KeyCode::Char(' ') => match state.cursor {
                0 => state.claude_checked = !state.claude_checked,
                _ => state.codex_checked = !state.codex_checked,
            },
            KeyCode::Enter => {
                let skill = state.skill.clone();
                let project_path = state.project_path.clone();
                let project_name = state.project_name.clone();
                let path = std::path::Path::new(&project_path);

                let mut unlinked = Vec::new();
                if state.claude_linked && !state.claude_checked {
                    linker::unlink_skill(path, &skill, crate::fs_util::Tool::Claude)?;
                    unlinked.push("Claude");
                }
                if state.codex_linked && !state.codex_checked {
                    linker::unlink_skill(path, &skill, crate::fs_util::Tool::Codex)?;
                    unlinked.push("Codex");
                }

                self.unlink_modal = None;
                if !unlinked.is_empty() {
                    self.status_msg = format!(
                        "Unlinked {} from {} ({})",
                        skill,
                        project_name,
                        unlinked.join(", ")
                    );
                    self.reload_all_project_links_all_tools();
                }
            }
            KeyCode::Esc => {
                self.unlink_modal = None;
            }
            _ => {}
        }
        Ok(())
    }

    // ── Delete Modal ──

    fn handle_delete_modal_key(&mut self, code: KeyCode) -> Result<()> {
        match code {
            KeyCode::Enter => {
                let skill = self.delete_modal.as_ref().unwrap().skill.clone();
                self.delete_modal = None;
                remover::remove_skill(&self.paths, &skill, &self.project_paths)?;
                self.status_msg = format!("Deleted {}", skill);
                self.reload()?;
            }
            KeyCode::Esc => {
                self.delete_modal = None;
            }
            _ => {}
        }
        Ok(())
    }

    // ── Screen 2: Projects ──

    fn handle_projects_screen_key(&mut self, code: KeyCode) -> Result<()> {
        match self.panel {
            Panel::Left | Panel::Middle => self.handle_project_list_key(code),
            Panel::Right => self.handle_tree_key(code),
        }
    }

    fn handle_tree_key(&mut self, code: KeyCode) -> Result<()> {
        let len = self.tree_rows.len();
        match code {
            KeyCode::Up | KeyCode::Char('k') => {
                self.projects_state.tree_cursor = self.projects_state.tree_cursor.saturating_sub(1);
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if len > 0 && self.projects_state.tree_cursor + 1 < len {
                    self.projects_state.tree_cursor += 1;
                }
            }
            KeyCode::Enter => {
                self.toggle_tree_collapse();
            }
            KeyCode::Char(' ') => {
                self.toggle_tree_link()?;
            }
            _ => {}
        }
        Ok(())
    }

    fn toggle_tree_collapse(&mut self) {
        let Some(row) = self.tree_rows.get(self.projects_state.tree_cursor) else {
            return;
        };
        match row {
            TreeRow::TagHeader { tag, .. } => {
                let tag = tag.clone();
                if !self.projects_state.collapsed_tags.remove(&tag) {
                    self.projects_state.collapsed_tags.insert(tag);
                }
                self.rebuild_tree();
            }
            TreeRow::UntaggedHeader { .. } => {
                let key = "__untagged__".to_string();
                if !self.projects_state.collapsed_tags.remove(&key) {
                    self.projects_state.collapsed_tags.insert(key);
                }
                self.rebuild_tree();
            }
            _ => {}
        }
    }

    fn toggle_tree_link(&mut self) -> Result<()> {
        let Some(project) = self.selected_project_path().map(|s| s.to_string()) else {
            self.status_msg = "No project selected".to_string();
            return Ok(());
        };
        let Some(row) = self.tree_rows.get(self.projects_state.tree_cursor).cloned() else {
            return Ok(());
        };

        let project_path = std::path::Path::new(&project);
        let project_name = crate::fs_util::basename(&project);

        match row {
            TreeRow::TagHeader { tag, .. } => {
                let skills = self.skills_for_tag(&tag);
                let all_linked = skills.iter().all(|s| self.is_skill_linked_to_selected(s));

                if all_linked {
                    for skill in skills {
                        linker::unlink_skill(project_path, skill, self.screen.tool())?;
                    }
                    self.status_msg = format!("Unlinked [{}] from {}", tag, project_name);
                } else {
                    let count = linker::link_by_tags(
                        &self.paths,
                        project_path,
                        std::slice::from_ref(&tag),
                        self.screen.tool(),
                    )?;
                    self.status_msg =
                        format!("Linked [{}] to {} ({} skills)", tag, project_name, count);
                }
                self.reload_project_links(&project);
                self.reload_all_project_links_all_tools();
            }
            TreeRow::Skill { skill } | TreeRow::UntaggedSkill { skill } => {
                if self.is_skill_linked_to_selected(&skill) {
                    linker::unlink_skill(project_path, &skill, self.screen.tool())?;
                    self.status_msg = format!("Unlinked {} from {}", skill, project_name);
                } else {
                    linker::link_skill(&self.paths, project_path, &skill, self.screen.tool())?;
                    self.status_msg = format!("Linked {} to {}", skill, project_name);
                }
                self.reload_project_links(&project);
                self.reload_all_project_links_all_tools();
            }
            TreeRow::UntaggedHeader { .. } => {}
        }
        Ok(())
    }

    fn handle_project_list_key(&mut self, code: KeyCode) -> Result<()> {
        let len = self.project_paths.len();
        match code {
            KeyCode::Up | KeyCode::Char('k') => {
                self.projects_state.selected_project =
                    self.projects_state.selected_project.saturating_sub(1);
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if len > 0 && self.projects_state.selected_project + 1 < len {
                    self.projects_state.selected_project += 1;
                }
            }
            _ => {}
        }
        Ok(())
    }

    // ── Text Input Modal ──

    fn handle_text_input_key(&mut self, code: KeyCode) -> Result<()> {
        let state = self.text_input.as_mut().unwrap();
        match code {
            KeyCode::Enter => {
                let new_tag = state.input.trim().to_string();
                self.text_input = None;
                if new_tag.is_empty() {
                    return Ok(());
                }
                if let Some(skill) = self
                    .skill_dirs
                    .get(self.skills_state.selected_skill)
                    .cloned()
                {
                    tagger::add_tag(&self.paths, &skill, &new_tag)?;
                    self.reload()?;
                    self.status_msg = format!("{}: added [{}]", skill, new_tag);
                }
            }
            KeyCode::Esc => {
                self.text_input = None;
            }
            KeyCode::Char(c) => {
                state.input.insert(state.cursor, c);
                state.cursor += c.len_utf8();
            }
            KeyCode::Backspace => {
                if state.cursor > 0 {
                    let prev = state.input[..state.cursor]
                        .char_indices()
                        .next_back()
                        .map(|(i, _)| i)
                        .unwrap_or(0);
                    state.input.replace_range(prev..state.cursor, "");
                    state.cursor = prev;
                }
            }
            KeyCode::Left => {
                if state.cursor > 0 {
                    state.cursor = state.input[..state.cursor]
                        .char_indices()
                        .next_back()
                        .map(|(i, _)| i)
                        .unwrap_or(0);
                }
            }
            KeyCode::Right => {
                if state.cursor < state.input.len() {
                    state.cursor += state.input[state.cursor..]
                        .chars()
                        .next()
                        .map(|c| c.len_utf8())
                        .unwrap_or(0);
                }
            }
            _ => {}
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crossterm::event::KeyModifiers;

    use super::*;
    use crate::fs_util::Tool;

    #[test]
    fn key_1_switches_to_skills_screen() {
        let mut app = App::new_test();
        app.screen = Screen::Claude;
        app.handle_key(KeyCode::Char('1'), KeyModifiers::NONE)
            .unwrap();
        assert_eq!(app.screen, Screen::Skills);
        assert_eq!(app.panel, Panel::Left);
    }

    #[test]
    fn key_2_switches_to_claude_screen() {
        let mut app = App::new_test();
        app.handle_key(KeyCode::Char('2'), KeyModifiers::NONE)
            .unwrap();
        assert_eq!(app.screen, Screen::Claude);
        assert_eq!(app.screen.tool(), Tool::Claude);
    }

    #[test]
    fn key_3_switches_to_codex_screen() {
        let mut app = App::new_test();
        app.handle_key(KeyCode::Char('3'), KeyModifiers::NONE)
            .unwrap();
        assert_eq!(app.screen, Screen::Codex);
        assert_eq!(app.screen.tool(), Tool::Codex);
    }

    #[test]
    fn skills_screen_right_arrow_cycles_left_middle_right() {
        let mut app = App::new_test();
        assert_eq!(app.panel, Panel::Left);
        app.handle_key(KeyCode::Right, KeyModifiers::NONE).unwrap();
        assert_eq!(app.panel, Panel::Middle);
        app.handle_key(KeyCode::Right, KeyModifiers::NONE).unwrap();
        assert_eq!(app.panel, Panel::Right);
        // clamp at Right
        app.handle_key(KeyCode::Right, KeyModifiers::NONE).unwrap();
        assert_eq!(app.panel, Panel::Right);
    }

    #[test]
    fn skills_screen_left_arrow_cycles_right_middle_left() {
        let mut app = App::new_test();
        app.panel = Panel::Right;
        app.handle_key(KeyCode::Left, KeyModifiers::NONE).unwrap();
        assert_eq!(app.panel, Panel::Middle);
        app.handle_key(KeyCode::Left, KeyModifiers::NONE).unwrap();
        assert_eq!(app.panel, Panel::Left);
        // clamp at Left
        app.handle_key(KeyCode::Left, KeyModifiers::NONE).unwrap();
        assert_eq!(app.panel, Panel::Left);
    }

    #[test]
    fn projects_screen_arrows_skip_middle() {
        let mut app = App::new_test();
        app.screen = Screen::Claude;
        app.panel = Panel::Left;
        app.handle_key(KeyCode::Right, KeyModifiers::NONE).unwrap();
        assert_eq!(app.panel, Panel::Right);
        app.handle_key(KeyCode::Left, KeyModifiers::NONE).unwrap();
        assert_eq!(app.panel, Panel::Left);
    }

    #[test]
    fn screen_tool_returns_correct_tool() {
        assert_eq!(Screen::Claude.tool(), Tool::Claude);
        assert_eq!(Screen::Codex.tool(), Tool::Codex);
    }

    #[test]
    fn screen_is_projects() {
        assert!(!Screen::Skills.is_projects());
        assert!(Screen::Claude.is_projects());
        assert!(Screen::Codex.is_projects());
    }

    #[test]
    fn q_key_quits() {
        let mut app = App::new_test();
        app.handle_key(KeyCode::Char('q'), KeyModifiers::NONE)
            .unwrap();
        assert!(app.should_quit);
    }

    #[test]
    fn esc_key_quits() {
        let mut app = App::new_test();
        app.handle_key(KeyCode::Esc, KeyModifiers::NONE).unwrap();
        assert!(app.should_quit);
    }

    #[test]
    fn esc_closes_text_input_without_quitting() {
        let mut app = App::new_test();
        app.text_input = Some(TextInputState {
            input: "test".into(),
            cursor: 4,
        });
        app.handle_key(KeyCode::Esc, KeyModifiers::NONE).unwrap();
        assert!(!app.should_quit);
        assert!(app.text_input.is_none());
    }

    #[test]
    fn text_input_ignores_screen_keys() {
        let mut app = App::new_test();
        app.text_input = Some(TextInputState {
            input: String::new(),
            cursor: 0,
        });
        app.handle_key(KeyCode::Char('2'), KeyModifiers::NONE)
            .unwrap();
        assert_eq!(app.screen, Screen::Skills);
    }
}
