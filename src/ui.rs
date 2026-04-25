use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph, Wrap},
    Frame,
};

use crate::app::{App, Screen};

pub fn draw(f: &mut Frame, app: &App) {
    let size = f.area();

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(0),
            Constraint::Length(3),
        ])
        .split(size);

    draw_header(f, app, chunks[0]);
    draw_body(f, app, chunks[1]);
    draw_footer(f, app, chunks[2]);
}

fn draw_header(f: &mut Frame, app: &App, area: Rect) {
    let title = format!(" DLsite TUI  |  {} ", app.status);
    let header = Paragraph::new(title)
        .style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))
        .block(Block::default().borders(Borders::ALL));
    f.render_widget(header, area);
}

fn draw_footer(f: &mut Frame, app: &App, area: Rect) {
    let hints = match &app.screen {
        Screen::Login { .. } => " Tab: next field | Enter: login | Ctrl+C: quit ",
        Screen::MainMenu { .. } => " ↑↓: navigate | Enter: select | f: fetch library | q: quit ",
        Screen::WorkList { .. } => " ↑↓/PgUp/PgDn: scroll | Enter: download | Esc: back ",
        Screen::CircleList { .. } => " ↑↓: scroll | Enter: select | Esc: back ",
        Screen::WorksInCircle { .. } => " ↑↓: scroll | Enter: download | Esc: back ",
        Screen::Confirm { .. } => " y/Enter: confirm | n/Esc: cancel ",
        Screen::Messages { .. } => " ↑↓: scroll | Esc/q: back ",
    };
    let footer = Paragraph::new(hints)
        .style(Style::default().fg(Color::DarkGray))
        .block(Block::default().borders(Borders::ALL));
    f.render_widget(footer, area);
}

fn draw_body(f: &mut Frame, app: &App, area: Rect) {
    match &app.screen {
        Screen::Login { username, password, focused_field, error } => {
            draw_login(f, area, username, password, *focused_field, error.as_deref());
        }
        Screen::MainMenu { selected } => {
            draw_main_menu(f, area, *selected);
        }
        Screen::WorkList { works, selected, scroll_offset, title } => {
            draw_work_list(f, area, works, *selected, *scroll_offset, title);
        }
        Screen::CircleList { circles, selected, scroll_offset } => {
            draw_circle_list(f, area, circles, *selected, *scroll_offset);
        }
        Screen::WorksInCircle { circle, works, selected, scroll_offset } => {
            draw_works_in_circle(f, area, circle, works, *selected, *scroll_offset);
        }
        Screen::Confirm { message, .. } => {
            draw_confirm(f, area, message);
        }
        Screen::Messages { lines, scroll_offset } => {
            draw_messages(f, area, lines, *scroll_offset);
        }
    }
}

fn draw_login(
    f: &mut Frame,
    area: Rect,
    username: &str,
    password: &str,
    focused_field: usize,
    error: Option<&str>,
) {
    let block = Block::default().title(" Login ").borders(Borders::ALL);
    f.render_widget(block, area);

    let inner = Rect {
        x: area.x + 2,
        y: area.y + 1,
        width: area.width.saturating_sub(4),
        height: area.height.saturating_sub(2),
    };

    let user_style = if focused_field == 0 {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default()
    };
    let pass_style = if focused_field == 1 {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default()
    };

    let password_masked: String = "*".repeat(password.len());

    let mut lines = vec![
        Line::from(vec![
            Span::raw("Username: "),
            Span::styled(username, user_style),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::raw("Password: "),
            Span::styled(password_masked.as_str(), pass_style),
        ]),
    ];

    if let Some(err) = error {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(err, Style::default().fg(Color::Red))));
    }

    let para = Paragraph::new(lines).wrap(Wrap { trim: false });
    f.render_widget(para, inner);
}

fn draw_main_menu(f: &mut Frame, area: Rect, selected: usize) {
    let items = vec![
        "1. Browse by ID",
        "2. Browse by Circle",
        "3. Browse by Recent Purchase",
        "4. Download All",
    ];

    let list_items: Vec<ListItem> = items
        .iter()
        .enumerate()
        .map(|(i, s)| {
            let style = if i == selected {
                Style::default().fg(Color::Black).bg(Color::Yellow)
            } else {
                Style::default()
            };
            ListItem::new(*s).style(style)
        })
        .collect();

    let mut state = ListState::default();
    state.select(Some(selected));

    let list = List::new(list_items)
        .block(Block::default().title(" Main Menu ").borders(Borders::ALL))
        .highlight_style(Style::default().fg(Color::Black).bg(Color::Yellow));

    f.render_stateful_widget(list, area, &mut state);
}

fn draw_work_list(
    f: &mut Frame,
    area: Rect,
    works: &[crate::db::Work],
    selected: usize,
    scroll_offset: usize,
    title: &str,
) {
    let height = area.height.saturating_sub(2) as usize;
    let visible = &works[scroll_offset..(scroll_offset + height).min(works.len())];

    let items: Vec<ListItem> = visible
        .iter()
        .enumerate()
        .map(|(i, w)| {
            let global_idx = i + scroll_offset;
            let style = if global_idx == selected {
                Style::default().fg(Color::Black).bg(Color::Cyan)
            } else {
                Style::default()
            };
            let text = format!("[{}] {} ({})", w.id, w.name, w.circle);
            ListItem::new(text).style(style)
        })
        .collect();

    let mut state = ListState::default();
    if selected >= scroll_offset {
        state.select(Some(selected - scroll_offset));
    }

    let list = List::new(items)
        .block(Block::default().title(format!(" {} ({} works) ", title, works.len())).borders(Borders::ALL))
        .highlight_style(Style::default().fg(Color::Black).bg(Color::Cyan));

    f.render_stateful_widget(list, area, &mut state);
}

fn draw_circle_list(
    f: &mut Frame,
    area: Rect,
    circles: &[(String, usize)],
    selected: usize,
    scroll_offset: usize,
) {
    let height = area.height.saturating_sub(2) as usize;
    let visible = &circles[scroll_offset..(scroll_offset + height).min(circles.len())];

    let items: Vec<ListItem> = visible
        .iter()
        .enumerate()
        .map(|(i, (name, count))| {
            let global_idx = i + scroll_offset;
            let style = if global_idx == selected {
                Style::default().fg(Color::Black).bg(Color::Cyan)
            } else {
                Style::default()
            };
            let text = format!("{} ({} works)", name, count);
            ListItem::new(text).style(style)
        })
        .collect();

    let mut state = ListState::default();
    if selected >= scroll_offset {
        state.select(Some(selected - scroll_offset));
    }

    let list = List::new(items)
        .block(Block::default().title(format!(" Circles ({} total) ", circles.len())).borders(Borders::ALL))
        .highlight_style(Style::default().fg(Color::Black).bg(Color::Cyan));

    f.render_stateful_widget(list, area, &mut state);
}

fn draw_works_in_circle(
    f: &mut Frame,
    area: Rect,
    circle: &str,
    works: &[crate::db::Work],
    selected: usize,
    scroll_offset: usize,
) {
    let height = area.height.saturating_sub(2) as usize;
    let visible = &works[scroll_offset..(scroll_offset + height).min(works.len())];

    let items: Vec<ListItem> = visible
        .iter()
        .enumerate()
        .map(|(i, w)| {
            let global_idx = i + scroll_offset;
            let style = if global_idx == selected {
                Style::default().fg(Color::Black).bg(Color::Cyan)
            } else {
                Style::default()
            };
            let text = format!("[{}] {}", w.id, w.name);
            ListItem::new(text).style(style)
        })
        .collect();

    let mut state = ListState::default();
    if selected >= scroll_offset {
        state.select(Some(selected - scroll_offset));
    }

    let list = List::new(items)
        .block(Block::default().title(format!(" {} - {} works ", circle, works.len())).borders(Borders::ALL))
        .highlight_style(Style::default().fg(Color::Black).bg(Color::Cyan));

    f.render_stateful_widget(list, area, &mut state);
}

fn draw_confirm(f: &mut Frame, area: Rect, message: &str) {
    let width = 60u16.min(area.width.saturating_sub(4));
    let height = 7u16;
    let x = area.x + (area.width.saturating_sub(width)) / 2;
    let y = area.y + (area.height.saturating_sub(height)) / 2;
    let popup_area = Rect { x, y, width, height };

    f.render_widget(Clear, popup_area);

    let text = format!("{}\n\n[y] Yes   [n] No", message);
    let para = Paragraph::new(text)
        .block(Block::default().title(" Confirm ").borders(Borders::ALL))
        .wrap(Wrap { trim: false });
    f.render_widget(para, popup_area);
}

fn draw_messages(f: &mut Frame, area: Rect, lines: &[String], scroll_offset: usize) {
    let height = area.height.saturating_sub(2) as usize;
    let visible: Vec<Line> = lines
        .iter()
        .skip(scroll_offset)
        .take(height)
        .map(|l| Line::from(l.as_str()))
        .collect();

    let para = Paragraph::new(visible)
        .block(Block::default().title(" Messages ").borders(Borders::ALL));
    f.render_widget(para, area);
}
