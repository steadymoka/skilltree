use ratatui::prelude::*;
use ratatui::widgets::*;

use super::app::{App, Screen};
use super::ui_projects;
use super::ui_skills;

pub fn render(frame: &mut Frame, app: &mut App) {
    let area = frame.area();

    let outer = Layout::vertical([
        Constraint::Length(1),
        Constraint::Min(6),
        Constraint::Length(1),
    ])
    .split(area);

    render_tab_bar(frame, outer[0], app);

    if app.screen.is_projects() {
        ui_projects::render(frame, outer[1], app);
    } else {
        ui_skills::render(frame, outer[1], app);
    }

    render_status_bar(frame, outer[2], app);

    if app.text_input.is_some() {
        render_text_input_modal(frame, area, app);
    }
    if app.unlink_modal.is_some() {
        render_unlink_modal(frame, area, app);
    }
    if app.delete_modal.is_some() {
        render_delete_modal(frame, area, app);
    }
}

fn render_tab_bar(frame: &mut Frame, area: Rect, app: &App) {
    let active = Style::new().fg(Color::White).bg(Color::Blue).bold();
    let inactive = Style::new().fg(Color::DarkGray);
    let tab_style = |s: Screen| if app.screen == s { active } else { inactive };

    let line = Line::from(vec![
        Span::styled(" Skill Tree ", Style::new().fg(Color::White).bold()),
        Span::styled(
            format!(
                "  {} skills  {} tags  ",
                app.skill_count(),
                app.all_tags.len()
            ),
            Style::new().fg(Color::DarkGray),
        ),
        Span::raw("  "),
        Span::styled(" 1:Skills ", tab_style(Screen::Skills)),
        Span::raw(" "),
        Span::styled(" 2:Claude ", tab_style(Screen::Claude)),
        Span::raw(" "),
        Span::styled(" 3:Codex ", tab_style(Screen::Codex)),
    ]);

    frame.render_widget(Paragraph::new(line), area);
}

fn render_status_bar(frame: &mut Frame, area: Rect, app: &App) {
    let help = if app.delete_modal.is_some() {
        " Enter:delete  Esc:cancel"
    } else if app.unlink_modal.is_some() {
        " Space:toggle  Enter:apply  Esc:cancel"
    } else if app.text_input.is_some() {
        " Enter:save  Esc:cancel"
    } else if app.screen.is_projects() {
        " 1/2/3:screen  \u{2190}\u{2192}:focus  \u{2191}\u{2193}:select  Space:link/unlink  Enter:fold  q:quit"
    } else {
        " 1/2/3:screen  \u{2190}\u{2192}:focus  \u{2191}\u{2193}:select  Space:toggle  a:new tag  d:delete  Esc:quit"
    };

    let line = if app.status_msg.is_empty() {
        Line::from(Span::styled(help, Style::new().fg(Color::DarkGray)))
    } else {
        Line::from(vec![
            Span::styled(&app.status_msg, Style::new().fg(Color::Yellow)),
            Span::styled("  \u{2502}  ", Style::new().fg(Color::DarkGray)),
            Span::styled(help, Style::new().fg(Color::DarkGray)),
        ])
    };

    frame.render_widget(Paragraph::new(line), area);
}

fn render_text_input_modal(frame: &mut Frame, area: Rect, app: &App) {
    let state = app.text_input.as_ref().unwrap();

    let width = 40u16.min(area.width.saturating_sub(4));
    let height = 3u16;
    let x = (area.width.saturating_sub(width)) / 2;
    let y = (area.height.saturating_sub(height)) / 2;
    let modal_area = Rect::new(x, y, width, height);

    frame.render_widget(Clear, modal_area);

    let block = Block::bordered()
        .title(" New Tag ")
        .border_type(BorderType::Rounded)
        .border_style(Style::new().fg(Color::Yellow));

    let inner = block.inner(modal_area);
    frame.render_widget(block, modal_area);

    let display = Line::from(vec![
        Span::raw(&state.input),
        Span::styled("\u{2588}", Style::new().fg(Color::Yellow)),
    ]);

    frame.render_widget(Paragraph::new(display), inner);
}

fn render_delete_modal(frame: &mut Frame, area: Rect, app: &App) {
    let state = app.delete_modal.as_ref().unwrap();

    let width = 36u16.min(area.width.saturating_sub(4));
    let height = 5u16;
    let x = (area.width.saturating_sub(width)) / 2;
    let y = (area.height.saturating_sub(height)) / 2;
    let modal_area = Rect::new(x, y, width, height);

    frame.render_widget(Clear, modal_area);

    let block = Block::bordered()
        .title(" Delete skill? ")
        .border_type(BorderType::Rounded)
        .border_style(Style::new().fg(Color::Red));

    let inner = block.inner(modal_area);
    frame.render_widget(block, modal_area);

    let lines = vec![
        Line::from(Span::styled(
            format!("  \"{}\"", state.skill),
            Style::new().fg(Color::Red).bold(),
        )),
        Line::from(""),
        Line::from(Span::styled(
            " Enter:delete  Esc:cancel",
            Style::new().fg(Color::DarkGray),
        )),
    ];

    frame.render_widget(Paragraph::new(lines), inner);
}

fn render_unlink_modal(frame: &mut Frame, area: Rect, app: &App) {
    let state = app.unlink_modal.as_ref().unwrap();

    let title = format!(" Unlink {} from {} ", state.skill, state.project_name);
    let width = (title.len() as u16 + 4)
        .max(34)
        .min(area.width.saturating_sub(4));
    let height = 7u16;
    let x = (area.width.saturating_sub(width)) / 2;
    let y = (area.height.saturating_sub(height)) / 2;
    let modal_area = Rect::new(x, y, width, height);

    frame.render_widget(Clear, modal_area);

    let block = Block::bordered()
        .title(title)
        .border_type(BorderType::Rounded)
        .border_style(Style::new().fg(Color::Yellow));

    let inner = block.inner(modal_area);
    frame.render_widget(block, modal_area);

    let cursor_marker = |idx: usize| {
        if state.cursor == idx {
            "\u{25b8} "
        } else {
            "  "
        }
    };

    let claude_check = if state.claude_checked {
        "[\u{2713}]"
    } else {
        "[ ]"
    };
    let codex_check = if state.codex_checked {
        "[\u{2713}]"
    } else {
        "[ ]"
    };

    let claude_style = if state.claude_linked {
        Style::new()
    } else {
        Style::new().fg(Color::DarkGray)
    };
    let codex_style = if state.codex_linked {
        Style::new()
    } else {
        Style::new().fg(Color::DarkGray)
    };

    let lines = vec![
        Line::from(vec![
            Span::styled(
                format!(" {}", cursor_marker(0)),
                Style::new().fg(Color::Yellow),
            ),
            Span::styled(format!("{} Claude", claude_check), claude_style),
        ]),
        Line::from(vec![
            Span::styled(
                format!(" {}", cursor_marker(1)),
                Style::new().fg(Color::Yellow),
            ),
            Span::styled(format!("{} Codex", codex_check), codex_style),
        ]),
        Line::from(""),
        Line::from(Span::styled(
            " Space:toggle  Enter:apply  Esc:cancel",
            Style::new().fg(Color::DarkGray),
        )),
    ];

    frame.render_widget(Paragraph::new(lines), inner);
}

const FOCUS_BORDER: Color = Color::Blue;
const FOCUS_HIGHLIGHT_BG: Color = Color::Blue;
const UNFOCUS_COLOR: Color = Color::DarkGray;

pub(super) struct PanelTheme {
    pub highlight_style: Style,
    pub highlight_symbol: &'static str,
    pub text_style: Style,
    border_style: Style,
}

impl PanelTheme {
    pub fn new(focused: bool) -> Self {
        if focused {
            Self {
                highlight_style: Style::new()
                    .bg(FOCUS_HIGHLIGHT_BG)
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
                highlight_symbol: " \u{25b8} ",
                text_style: Style::new(),
                border_style: Style::new().fg(FOCUS_BORDER),
            }
        } else {
            Self {
                highlight_style: Style::new().bg(UNFOCUS_COLOR).fg(Color::White),
                highlight_symbol: "   ",
                text_style: Style::new().fg(UNFOCUS_COLOR),
                border_style: Style::new().fg(UNFOCUS_COLOR),
            }
        }
    }

    pub fn block<'a>(&self, title: &'a str) -> Block<'a> {
        Block::bordered()
            .title(title)
            .border_style(self.border_style)
            .border_type(BorderType::Rounded)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn focused_theme_uses_accent_colors() {
        let theme = PanelTheme::new(true);
        assert_eq!(theme.highlight_symbol, " \u{25b8} ");
        assert_eq!(theme.highlight_style.bg, Some(FOCUS_HIGHLIGHT_BG));
        assert_eq!(theme.border_style.fg, Some(FOCUS_BORDER));
        assert_eq!(theme.text_style, Style::new());
    }

    #[test]
    fn unfocused_theme_is_uniformly_dimmed() {
        let theme = PanelTheme::new(false);
        assert_eq!(theme.highlight_symbol, "   ");
        assert_eq!(theme.highlight_style.bg, Some(UNFOCUS_COLOR));
        assert_eq!(theme.text_style.fg, Some(UNFOCUS_COLOR));
        assert_eq!(theme.border_style.fg, Some(UNFOCUS_COLOR));
    }

    #[test]
    fn block_creates_rounded_bordered_block() {
        let theme = PanelTheme::new(true);
        let _block = theme.block(" Test ");
    }
}
