use ratatui::prelude::*;
use ratatui::widgets::*;

use super::app::{App, Panel};
use super::ui::PanelTheme;

pub fn render(frame: &mut Frame, area: Rect, app: &mut App) {
    let chunks = Layout::horizontal([
        Constraint::Percentage(43),
        Constraint::Percentage(28),
        Constraint::Percentage(29),
    ])
    .split(area);

    render_skill_list(frame, chunks[0], app);
    render_tag_checkboxes(frame, chunks[1], app);
    render_skill_projects(frame, chunks[2], app);
}

fn render_skill_list(frame: &mut Frame, area: Rect, app: &mut App) {
    let focused = app.panel == Panel::Left;
    let theme = PanelTheme::new(focused);

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

            let count = app.skill_linked_project_count(skill);
            let count_span = if count > 0 {
                format!("  ({})", count)
            } else {
                String::new()
            };

            ListItem::new(Line::from(vec![
                Span::styled(format!("  {}", skill), theme.text_style),
                Span::styled(tag_str, Style::new().fg(Color::DarkGray)),
                Span::styled(count_span, Style::new().fg(Color::Cyan)),
            ]))
        })
        .collect();

    let list = List::new(items)
        .block(theme.block(" Skills "))
        .highlight_style(theme.highlight_style)
        .highlight_symbol(theme.highlight_symbol);

    frame.render_stateful_widget(list, area, &mut app.skills_state.skill_list_state);
}

fn render_tag_checkboxes(frame: &mut Frame, area: Rect, app: &mut App) {
    let focused = app.panel == Panel::Middle;
    let theme = PanelTheme::new(focused);

    if app.all_tags.is_empty() {
        let msg = Paragraph::new(Line::from(vec![Span::styled(
            "  No tags yet. Press 'a' to create one.",
            Style::new().fg(Color::DarkGray),
        )]))
        .block(theme.block(" Tags "));
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
                Span::styled(tag.as_str(), theme.text_style),
            ]))
        })
        .collect();

    let list = List::new(items)
        .block(theme.block(" Tags "))
        .highlight_style(theme.highlight_style)
        .highlight_symbol(theme.highlight_symbol);

    frame.render_stateful_widget(list, area, &mut app.skills_state.tag_list_state);
}

fn render_skill_projects(frame: &mut Frame, area: Rect, app: &mut App) {
    let focused = app.panel == Panel::Right;
    let theme = PanelTheme::new(focused);

    let entries = app.selected_skill_project_entries();

    if entries.is_empty() {
        let msg = Paragraph::new(Line::from(vec![Span::styled(
            "  No projects linked",
            Style::new().fg(Color::DarkGray),
        )]))
        .block(theme.block(" Projects "));
        frame.render_widget(msg, area);
        return;
    }

    let items: Vec<ListItem> = entries
        .iter()
        .map(|entry| {
            let mut spans = vec![Span::styled(
                format!("  {}", entry.project_name),
                theme.text_style,
            )];

            let tools: Vec<Span> = [
                entry
                    .has_claude
                    .then(|| Span::styled("C", Style::new().fg(Color::Cyan))),
                entry
                    .has_codex
                    .then(|| Span::styled("X", Style::new().fg(Color::Yellow))),
            ]
            .into_iter()
            .flatten()
            .collect();

            if !tools.is_empty() {
                spans.push(Span::styled("  ", Style::new()));
                for (i, tool_span) in tools.into_iter().enumerate() {
                    if i > 0 {
                        spans.push(Span::styled("\u{00b7}", Style::new().fg(Color::DarkGray)));
                    }
                    spans.push(tool_span);
                }
            }

            ListItem::new(Line::from(spans))
        })
        .collect();

    let list = List::new(items)
        .block(theme.block(" Projects "))
        .highlight_style(theme.highlight_style)
        .highlight_symbol(theme.highlight_symbol);

    frame.render_stateful_widget(list, area, &mut app.skills_state.project_link_list_state);
}
