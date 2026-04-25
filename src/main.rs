mod api;
mod app;
mod config;
mod db;
mod ui;

use anyhow::Result;
use app::{App, ConfirmAction, Screen};
use crossterm::{
    event::{self, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use db::Work;
use ratatui::{backend::CrosstermBackend, Terminal};
use std::{
    io,
    path::PathBuf,
    sync::{mpsc, Arc},
    time::Duration,
};

fn main() -> Result<()> {
    let config = config::load_config()?;
    let api = Arc::new(api::ApiClient::new()?);
    let db = db::open_db()?;

    let (msg_tx, msg_rx) = mpsc::channel::<String>();

    let mut app = App::new(api, db, config, msg_tx, msg_rx);

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let original_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let _ = disable_raw_mode();
        let _ = execute!(io::stdout(), LeaveAlternateScreen);
        original_hook(info);
    }));

    let result = run_app(&mut terminal, &mut app);

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    if let Err(e) = result {
        eprintln!("Error: {}", e);
    }

    Ok(())
}

fn run_app<B: ratatui::backend::Backend>(
    terminal: &mut Terminal<B>,
    app: &mut App,
) -> Result<()> {
    loop {
        terminal.draw(|f| ui::draw(f, app))?;

        // Poll messages from background threads
        while let Ok(msg) = app.msg_rx.try_recv() {
            match &mut app.screen {
                Screen::Messages { lines, .. } => {
                    lines.push(msg);
                }
                _ => {
                    app.status = msg;
                }
            }
        }

        if !event::poll(Duration::from_millis(50))? {
            continue;
        }

        if let Event::Key(key) = event::read()? {
            if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
                return Ok(());
            }

            // Determine which screen we're on without holding a mutable borrow
            enum ScreenTag {
                Login,
                MainMenu,
                WorkList,
                CircleList,
                WorksInCircle,
                Confirm,
                Messages,
            }

            let tag = match &app.screen {
                Screen::Login { .. } => ScreenTag::Login,
                Screen::MainMenu { .. } => ScreenTag::MainMenu,
                Screen::WorkList { .. } => ScreenTag::WorkList,
                Screen::CircleList { .. } => ScreenTag::CircleList,
                Screen::WorksInCircle { .. } => ScreenTag::WorksInCircle,
                Screen::Confirm { .. } => ScreenTag::Confirm,
                Screen::Messages { .. } => ScreenTag::Messages,
            };

            match tag {
                ScreenTag::Login => {
                    handle_login_key(app, key);
                }
                ScreenTag::MainMenu => {
                    let sel = if let Screen::MainMenu { selected } = &app.screen { *selected } else { 0 };
                    match key.code {
                        KeyCode::Char('q') => return Ok(()),
                        KeyCode::Up => {
                            if let Screen::MainMenu { selected } = &mut app.screen {
                                if *selected > 0 { *selected -= 1; }
                            }
                        }
                        KeyCode::Down => {
                            if let Screen::MainMenu { selected } = &mut app.screen {
                                if *selected < 3 { *selected += 1; }
                            }
                        }
                        KeyCode::Enter => {
                            handle_main_menu_enter(app, sel);
                        }
                        KeyCode::Char('f') => {
                            fetch_library(app);
                        }
                        _ => {}
                    }
                }
                ScreenTag::WorkList => {
                    let (sel, _off, len) = if let Screen::WorkList { selected, scroll_offset, works, .. } = &app.screen {
                        (*selected, *scroll_offset, works.len())
                    } else { (0, 0, 0) };
                    match key.code {
                        KeyCode::Esc => {
                            app.screen = Screen::MainMenu { selected: 0 };
                        }
                        KeyCode::Up => {
                            if let Screen::WorkList { selected, scroll_offset, .. } = &mut app.screen {
                                if *selected > 0 {
                                    *selected -= 1;
                                    if *selected < *scroll_offset {
                                        *scroll_offset = *selected;
                                    }
                                }
                            }
                        }
                        KeyCode::Down => {
                            if sel + 1 < len {
                                if let Screen::WorkList { selected, scroll_offset, .. } = &mut app.screen {
                                    *selected += 1;
                                    if *selected >= *scroll_offset + 30 {
                                        *scroll_offset = selected.saturating_sub(29);
                                    }
                                }
                            }
                        }
                        KeyCode::PageUp => {
                            if let Screen::WorkList { selected, scroll_offset, .. } = &mut app.screen {
                                *selected = selected.saturating_sub(10);
                                *scroll_offset = scroll_offset.saturating_sub(10);
                            }
                        }
                        KeyCode::PageDown => {
                            if let Screen::WorkList { selected, scroll_offset, works, .. } = &mut app.screen {
                                let l = works.len();
                                *selected = (*selected + 10).min(l.saturating_sub(1));
                                *scroll_offset = (*scroll_offset + 10).min(l.saturating_sub(1));
                            }
                        }
                        KeyCode::Enter => {
                            let info = if let Screen::WorkList { works, selected, .. } = &app.screen {
                                works.get(*selected).map(|w| (w.id.clone(), w.name.clone()))
                            } else { None };
                            if let Some((id, name)) = info {
                                app.screen = Screen::Confirm {
                                    message: format!("Download {}?", name),
                                    action: ConfirmAction::DownloadWork(id),
                                };
                            }
                        }
                        _ => {}
                    }
                }
                ScreenTag::CircleList => {
                    let (sel, len) = if let Screen::CircleList { selected, circles, .. } = &app.screen {
                        (*selected, circles.len())
                    } else { (0, 0) };
                    match key.code {
                        KeyCode::Esc => {
                            app.screen = Screen::MainMenu { selected: 0 };
                        }
                        KeyCode::Up => {
                            if let Screen::CircleList { selected, scroll_offset, .. } = &mut app.screen {
                                if *selected > 0 {
                                    *selected -= 1;
                                    if *selected < *scroll_offset {
                                        *scroll_offset = *selected;
                                    }
                                }
                            }
                        }
                        KeyCode::Down => {
                            if sel + 1 < len {
                                if let Screen::CircleList { selected, scroll_offset, .. } = &mut app.screen {
                                    *selected += 1;
                                    if *selected >= *scroll_offset + 30 {
                                        *scroll_offset = selected.saturating_sub(29);
                                    }
                                }
                            }
                        }
                        KeyCode::Enter => {
                            let circle_name = if let Screen::CircleList { circles, selected, .. } = &app.screen {
                                circles.get(*selected).map(|(n, _)| n.clone())
                            } else { None };
                            if let Some(circle) = circle_name {
                                match db::get_works_by_circle(&app.db, &circle) {
                                    Ok(works) => {
                                        app.screen = Screen::WorksInCircle {
                                            circle,
                                            works,
                                            selected: 0,
                                            scroll_offset: 0,
                                        };
                                    }
                                    Err(e) => {
                                        app.status = format!("Error: {}", e);
                                    }
                                }
                            }
                        }
                        _ => {}
                    }
                }
                ScreenTag::WorksInCircle => {
                    let (sel, len) = if let Screen::WorksInCircle { selected, works, .. } = &app.screen {
                        (*selected, works.len())
                    } else { (0, 0) };
                    match key.code {
                        KeyCode::Esc => {
                            let circles = db::get_all_circles(&app.db).unwrap_or_default();
                            app.screen = Screen::CircleList {
                                circles,
                                selected: 0,
                                scroll_offset: 0,
                            };
                        }
                        KeyCode::Up => {
                            if let Screen::WorksInCircle { selected, scroll_offset, .. } = &mut app.screen {
                                if *selected > 0 {
                                    *selected -= 1;
                                    if *selected < *scroll_offset {
                                        *scroll_offset = *selected;
                                    }
                                }
                            }
                        }
                        KeyCode::Down => {
                            if sel + 1 < len {
                                if let Screen::WorksInCircle { selected, scroll_offset, .. } = &mut app.screen {
                                    *selected += 1;
                                    if *selected >= *scroll_offset + 30 {
                                        *scroll_offset = selected.saturating_sub(29);
                                    }
                                }
                            }
                        }
                        KeyCode::Enter => {
                            let info = if let Screen::WorksInCircle { works, selected, .. } = &app.screen {
                                works.get(*selected).map(|w| (w.id.clone(), w.name.clone()))
                            } else { None };
                            if let Some((id, name)) = info {
                                app.screen = Screen::Confirm {
                                    message: format!("Download {}?", name),
                                    action: ConfirmAction::DownloadWork(id),
                                };
                            }
                        }
                        _ => {}
                    }
                }
                ScreenTag::Confirm => {
                    let action_opt = if let Screen::Confirm { action, .. } = &app.screen {
                        Some(action.clone())
                    } else { None };
                    if let Some(action) = action_opt {
                        match key.code {
                            KeyCode::Char('y') | KeyCode::Enter => {
                                handle_confirm(app, action);
                            }
                            KeyCode::Char('n') | KeyCode::Esc => {
                                app.screen = Screen::MainMenu { selected: 0 };
                            }
                            _ => {}
                        }
                    }
                }
                ScreenTag::Messages => {
                    let (off, len) = if let Screen::Messages { scroll_offset, lines } = &app.screen {
                        (*scroll_offset, lines.len())
                    } else { (0, 0) };
                    match key.code {
                        KeyCode::Esc | KeyCode::Char('q') => {
                            app.screen = Screen::MainMenu { selected: 0 };
                        }
                        KeyCode::Up => {
                            if let Screen::Messages { scroll_offset, .. } = &mut app.screen {
                                *scroll_offset = scroll_offset.saturating_sub(1);
                            }
                        }
                        KeyCode::Down => {
                            if off + 1 < len {
                                if let Screen::Messages { scroll_offset, .. } = &mut app.screen {
                                    *scroll_offset += 1;
                                }
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
    }
}

fn handle_login_key(app: &mut App, key: crossterm::event::KeyEvent) {
    // Extract needed data first to avoid borrow issues
    let (username, password, _focused_field) = if let Screen::Login { username, password, focused_field, .. } = &app.screen {
        (username.clone(), password.clone(), *focused_field)
    } else {
        return;
    };

    match key.code {
        KeyCode::Tab | KeyCode::Down => {
            if let Screen::Login { focused_field, .. } = &mut app.screen {
                *focused_field = (*focused_field + 1) % 2;
            }
        }
        KeyCode::Up => {
            if let Screen::Login { focused_field, .. } = &mut app.screen {
                *focused_field = if *focused_field == 0 { 1 } else { 0 };
            }
        }
        KeyCode::Backspace => {
            if let Screen::Login { username, password, focused_field, .. } = &mut app.screen {
                if *focused_field == 0 {
                    username.pop();
                } else {
                    password.pop();
                }
            }
        }
        KeyCode::Char(c) => {
            if let Screen::Login { username, password, focused_field, .. } = &mut app.screen {
                if *focused_field == 0 {
                    username.push(c);
                } else {
                    password.push(c);
                }
            }
        }
        KeyCode::Enter => {
            let api = app.api.clone();
            match api.login(&username, &password) {
                Ok(()) => {
                    let _ = api.save_cookies();
                    app.config.username = Some(username);
                    let _ = config::save_config(&app.config);
                    app.screen = Screen::MainMenu { selected: 0 };
                    app.status = String::from("Logged in successfully");
                }
                Err(e) => {
                    if let Screen::Login { error, .. } = &mut app.screen {
                        *error = Some(format!("Login failed: {}", e));
                    }
                }
            }
        }
        _ => {}
    }
}

fn handle_main_menu_enter(app: &mut App, selected: usize) {
    match selected {
        0 => {
            match db::get_all_works(&app.db) {
                Ok(works) => {
                    app.screen = Screen::WorkList {
                        works,
                        selected: 0,
                        scroll_offset: 0,
                        title: String::from("All Works by ID"),
                    };
                }
                Err(e) => {
                    app.status = format!("Error: {}", e);
                }
            }
        }
        1 => {
            match db::get_all_circles(&app.db) {
                Ok(circles) => {
                    app.screen = Screen::CircleList {
                        circles,
                        selected: 0,
                        scroll_offset: 0,
                    };
                }
                Err(e) => {
                    app.status = format!("Error: {}", e);
                }
            }
        }
        2 => {
            match db::get_all_works_by_date(&app.db) {
                Ok(works) => {
                    app.screen = Screen::WorkList {
                        works,
                        selected: 0,
                        scroll_offset: 0,
                        title: String::from("Recent Purchases"),
                    };
                }
                Err(e) => {
                    app.status = format!("Error: {}", e);
                }
            }
        }
        3 => {
            app.screen = Screen::Confirm {
                message: String::from("Download all works?"),
                action: ConfirmAction::DownloadAll,
            };
        }
        _ => {}
    }
}

fn handle_confirm(app: &mut App, action: ConfirmAction) {
    match action {
        ConfirmAction::DownloadWork(work_id) => {
            let api = app.api.clone();
            let tx = app.msg_tx.clone();
            let id = work_id.clone();
            std::thread::spawn(move || {
                download_work(&api, &tx, &id, &PathBuf::from("."));
            });
            app.screen = Screen::Messages {
                lines: vec![format!("Downloading {}...", work_id)],
                scroll_offset: 0,
            };
        }
        ConfirmAction::DownloadCircle(circle) => {
            let works = db::get_works_by_circle(&app.db, &circle).unwrap_or_default();
            let api = app.api.clone();
            let tx = app.msg_tx.clone();
            let circle_clone = circle.clone();
            std::thread::spawn(move || {
                for w in &works {
                    download_work(&api, &tx, &w.id, &PathBuf::from("."));
                }
                let _ = tx.send(format!("Done downloading circle: {}", circle_clone));
            });
            app.screen = Screen::Messages {
                lines: vec![format!("Downloading circle {}...", circle)],
                scroll_offset: 0,
            };
        }
        ConfirmAction::DownloadAll => {
            let works = db::get_all_works(&app.db).unwrap_or_default();
            let api = app.api.clone();
            let tx = app.msg_tx.clone();
            std::thread::spawn(move || {
                for w in &works {
                    download_work(&api, &tx, &w.id, &PathBuf::from("."));
                }
                let _ = tx.send(String::from("Done downloading all works"));
            });
            app.screen = Screen::Messages {
                lines: vec![String::from("Downloading all works...")],
                scroll_offset: 0,
            };
        }
        ConfirmAction::FetchLibrary => {
            fetch_library(app);
        }
    }
}

fn fetch_library(app: &mut App) {
    let api = app.api.clone();
    let tx = app.msg_tx.clone();
    std::thread::spawn(move || {
        let _ = tx.send(String::from("Fetching library count..."));
        match api.fetch_library_count() {
            Err(e) => {
                let _ = tx.send(format!("Error fetching count: {}", e));
            }
            Ok((user_count, page_limit)) => {
                let _ = tx.send(format!("Total works: {}, pages: {}", user_count, page_limit));
                let conn = match db::open_db() {
                    Ok(c) => c,
                    Err(e) => {
                        let _ = tx.send(format!("DB error: {}", e));
                        return;
                    }
                };
                for page in 1..=page_limit {
                    let _ = tx.send(format!("Fetching page {}/{}...", page, page_limit));
                    match api.fetch_purchased_page(page) {
                        Err(e) => {
                            let _ = tx.send(format!("Error fetching page {}: {}", page, e));
                        }
                        Ok(items) => {
                            for (workno, sales_date) in items {
                                match api.fetch_work_info(&workno) {
                                    Err(e) => {
                                        let _ = tx.send(format!("Error fetching info for {}: {}", workno, e));
                                    }
                                    Ok(info) => {
                                        let work = Work {
                                            id: workno.clone(),
                                            name: info.work_name.clone(),
                                            circle: info.maker_name.clone(),
                                            file_count: info.contents.len() as i64,
                                            purchase_date: sales_date.clone(),
                                        };
                                        if let Err(e) = db::upsert_work(&conn, &work) {
                                            let _ = tx.send(format!("DB error for {}: {}", workno, e));
                                        } else {
                                            let _ = tx.send(format!("Saved: {} - {}", workno, info.work_name));
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                let _ = tx.send(String::from("Library fetch complete!"));
            }
        }
    });

    app.screen = Screen::Messages {
        lines: vec![String::from("Fetching library...")],
        scroll_offset: 0,
    };
}

fn download_work(api: &api::ApiClient, tx: &std::sync::mpsc::Sender<String>, work_id: &str, output_dir: &PathBuf) {
    let _ = tx.send(format!("Fetching info for {}...", work_id));
    match api.fetch_work_info(work_id) {
        Err(e) => {
            let _ = tx.send(format!("Error fetching info for {}: {}", work_id, e));
        }
        Ok(info) => {
            let work_dir = output_dir.join(work_id);
            for (i, file) in info.contents.iter().enumerate() {
                let _ = tx.send(format!("Downloading file {}/{}: {}", i + 1, info.contents.len(), file.file_name));
                match api.download_file(work_id, i + 1, &file.file_name, &work_dir) {
                    Err(e) => {
                        let _ = tx.send(format!("Error downloading {}: {}", file.file_name, e));
                    }
                    Ok(bytes) => {
                        let _ = tx.send(format!("Downloaded {} ({} bytes)", file.file_name, bytes));
                    }
                }
            }
        }
    }
}
