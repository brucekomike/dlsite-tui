use std::sync::mpsc::{Receiver, Sender};
use std::sync::Arc;

use crate::api::ApiClient;
use crate::config::Config;
use crate::db::Work;
use rusqlite::Connection;

#[derive(Debug, Clone)]
pub enum ConfirmAction {
    DownloadWork(String),
    DownloadCircle(String),
    DownloadAll,
    FetchLibrary,
}

pub enum Screen {
    Login {
        username: String,
        password: String,
        focused_field: usize,
        error: Option<String>,
    },
    MainMenu {
        selected: usize,
    },
    WorkList {
        works: Vec<Work>,
        selected: usize,
        scroll_offset: usize,
        title: String,
    },
    CircleList {
        circles: Vec<(String, usize)>,
        selected: usize,
        scroll_offset: usize,
    },
    WorksInCircle {
        circle: String,
        works: Vec<Work>,
        selected: usize,
        scroll_offset: usize,
    },
    Confirm {
        message: String,
        action: ConfirmAction,
    },
    Messages {
        lines: Vec<String>,
        scroll_offset: usize,
    },
}

pub struct App {
    pub screen: Screen,
    pub status: String,
    pub api: Arc<ApiClient>,
    pub db: Connection,
    pub config: Config,
    pub msg_tx: Sender<String>,
    pub msg_rx: Receiver<String>,
}

impl App {
    pub fn new(
        api: Arc<ApiClient>,
        db: Connection,
        config: Config,
        msg_tx: Sender<String>,
        msg_rx: Receiver<String>,
    ) -> Self {
        let screen = if api.is_logged_in() {
            Screen::MainMenu { selected: 0 }
        } else {
            Screen::Login {
                username: config.username.clone().unwrap_or_default(),
                password: String::new(),
                focused_field: 0,
                error: None,
            }
        };
        App {
            screen,
            status: String::from("Ready"),
            api,
            db,
            config,
            msg_tx,
            msg_rx,
        }
    }
}
