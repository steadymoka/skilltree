use ratatui::prelude::*;
use ratatui::widgets::*;

use super::app::{App, Panel};
use super::ui::panel_block;

pub fn render(frame: &mut Frame, area: Rect, app: &mut App) {
    let chunks =
        Layout::horizontal([Constraint::Percentage(50), Constraint::Percentage(50)]).split(area);

    render_skill_list(frame, chunks[0], app);
    render_tag_checkboxes(frame, chunks[1], app);
}

fn render_skill_list(frame: &mut Frame, area: Rect, app: &mut App) {
    let focused = app.panel == Panel::Left;

    let items: Vec<ListItem> = app
        .skill_dirs
        .iter()
        .map(|skill| {
            let tags = app.tags_for_skill(skill);
            let tag_str = if tags.is_empty() {
                String::new()
            } else {
                format!("  [{}]", tags.join(", "))
            };

            ListItem::new(Line::from(vec![
                Span::styled(format!("  {}", skill), Style::new()),
                Span::styled(tag_str, Style::new().fg(Color::DarkGray)),
            ]))
        })
        .collect();

    let list = List::new(items)
        .block(panel_block(" Skills ", focused))
        .highlight_style(
            Style::new()
                .bg(Color::Blue)
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol(" \u{25b8} ");

    frame.render_stateful_widget(list, area, &mut app.skills_state.skill_list_state);
}

fn render_tag_checkboxes(frame: &mut Frame, area: Rect, app: &mut App) {
    let focused = app.panel == Panel::Right;

    if app.all_tags.is_empty() {
        let block = panel_block(" Tags ", focused);
        let msg = Paragraph::new(Line::from(vec![Span::styled(
            "  No tags yet. Press 'a' to create one.",
            Style::new().fg(Color::DarkGray),
        )]))
        .block(block);
        frame.render_widget(msg, area);
        return;
    }

    let selected_skill = app.skill_dirs.get(app.skills_state.selected_skill).cloned();
    let skill_tags: Vec<String> = selected_skill
        .as_deref()
        .map(|s| app.tags_for_skill(s).to_vec())
        .unwrap_or_default();

    let items: Vec<ListItem> = app
        .all_tags
        .iter()
        .map(|tag| {
            let checked = skill_tags.contains(tag);
            let marker = if checked { "[\u{2713}]" } else { "[ ]" };
            let marker_style = if checked {
                Style::new().fg(Color::Green)
            } else {
                Style::new().fg(Color::DarkGray)
            };

            ListItem::new(Line::from(vec![
                Span::styled(format!("  {} ", marker), marker_style),
                Span::styled(tag.as_str(), Style::new()),
            ]))
        })
        .collect();

    let list = List::new(items)
        .block(panel_block(" Tags ", focused))
        .highlight_style(
            Style::new()
                .bg(Color::Blue)
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol(" \u{25b8} ");

    frame.render_stateful_widget(list, area, &mut app.skills_state.tag_list_state);
}
