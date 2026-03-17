use ratatui::prelude::*;
use ratatui::widgets::*;

use super::app::{App, Panel, TreeRow};
use super::ui::PanelTheme;

pub fn render(frame: &mut Frame, area: Rect, app: &mut App) {
    let chunks =
        Layout::horizontal([Constraint::Percentage(40), Constraint::Percentage(60)]).split(area);

    render_project_list(frame, chunks[0], app);
    render_tree_view(frame, chunks[1], app);
}

fn render_tree_view(frame: &mut Frame, area: Rect, app: &mut App) {
    let focused = app.panel == Panel::Right;
    let theme = PanelTheme::new(focused);

    if app.tree_rows.is_empty() {
        let msg = Paragraph::new(Line::from(vec![Span::styled(
            "  No skills found.",
            Style::new().fg(Color::DarkGray),
        )]))
        .block(theme.block(" Skills by Tag "));
        frame.render_widget(msg, area);
        return;
    }

    let items: Vec<ListItem> = app
        .tree_rows
        .iter()
        .map(|row| match row {
            TreeRow::TagHeader { tag, skill_count } => {
                let collapsed = app.projects_state.collapsed_tags.contains(tag);
                let arrow = if collapsed { "\u{25b8}" } else { "\u{25be}" };

                let skills = app.skills_for_tag(tag);
                let all_linked =
                    !skills.is_empty() && skills.iter().all(|s| app.is_skill_linked_to_selected(s));
                let some_linked = skills.iter().any(|s| app.is_skill_linked_to_selected(s));

                let check = if all_linked {
                    "[\u{2713}]"
                } else if some_linked {
                    "[-]"
                } else {
                    "[ ]"
                };

                let check_style = if all_linked {
                    Style::new().fg(Color::Green).bold()
                } else if some_linked {
                    Style::new().fg(Color::Yellow)
                } else {
                    Style::new().fg(Color::DarkGray)
                };

                ListItem::new(Line::from(vec![
                    Span::styled(format!(" {} ", arrow), theme.text_style),
                    Span::styled(format!("{} ", check), check_style),
                    Span::styled(tag.as_str(), theme.text_style.bold()),
                    Span::styled(
                        format!("  {}", skill_count),
                        Style::new().fg(Color::DarkGray),
                    ),
                ]))
            }
            TreeRow::Skill { skill } => {
                let linked = app.is_skill_linked_to_selected(skill);
                let check = if linked { "[\u{2713}]" } else { "[ ]" };
                let check_style = if linked {
                    Style::new().fg(Color::Green)
                } else {
                    Style::new().fg(Color::DarkGray)
                };

                ListItem::new(Line::from(vec![
                    Span::styled("     ", Style::new()),
                    Span::styled(format!("{} ", check), check_style),
                    Span::styled(skill.as_str(), theme.text_style),
                ]))
            }
            TreeRow::UntaggedHeader { skill_count } => {
                let collapsed = app.projects_state.collapsed_tags.contains("__untagged__");
                let arrow = if collapsed { "\u{25b8}" } else { "\u{25be}" };

                ListItem::new(Line::from(vec![
                    Span::styled(format!(" {} ", arrow), Style::new().fg(Color::DarkGray)),
                    Span::styled(
                        format!("no tag  {}", skill_count),
                        Style::new().fg(Color::DarkGray).italic(),
                    ),
                ]))
            }
            TreeRow::UntaggedSkill { skill } => {
                let linked = app.is_skill_linked_to_selected(skill);
                let check = if linked { "[\u{2713}]" } else { "[ ]" };
                let check_style = if linked {
                    Style::new().fg(Color::Green)
                } else {
                    Style::new().fg(Color::DarkGray)
                };

                ListItem::new(Line::from(vec![
                    Span::styled("     ", Style::new()),
                    Span::styled(format!("{} ", check), check_style),
                    Span::styled(skill.as_str(), Style::new().fg(Color::DarkGray)),
                ]))
            }
        })
        .collect();

    let list = List::new(items)
        .block(theme.block(" Skills by Tag "))
        .highlight_style(theme.highlight_style);

    frame.render_stateful_widget(list, area, &mut app.projects_state.tree_list_state);
}

fn render_project_list(frame: &mut Frame, area: Rect, app: &mut App) {
    let focused = matches!(app.panel, Panel::Left | Panel::Middle);
    let theme = PanelTheme::new(focused);

    if app.project_paths.is_empty() {
        let msg = Paragraph::new(Line::from(vec![Span::styled(
            "  No projects found",
            Style::new().fg(Color::DarkGray),
        )]))
        .block(theme.block(" Projects "));
        frame.render_widget(msg, area);
        return;
    }

    let items: Vec<ListItem> = app
        .project_paths
        .iter()
        .map(|path| {
            let name = app.display_project_name(path);
            let count = app.linked_count(path);

            let count_style = if count > 0 {
                Style::new().fg(Color::Green)
            } else {
                Style::new().fg(Color::DarkGray)
            };

            ListItem::new(Line::from(vec![
                Span::styled(format!("  {}", name), theme.text_style),
                Span::styled(format!("  {} linked", count), count_style),
            ]))
        })
        .collect();

    let list = List::new(items)
        .block(theme.block(" Projects "))
        .highlight_style(theme.highlight_style)
        .highlight_symbol(theme.highlight_symbol);

    frame.render_stateful_widget(list, area, &mut app.projects_state.project_list_state);
}
