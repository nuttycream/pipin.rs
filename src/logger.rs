use axum::response::Html;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::AppState;

#[derive(Clone)]
pub enum LogType {
    Info,
    Error,
}

#[derive(Clone)]
pub struct LogEntry {
    pub log_type: LogType,
    pub message: String,
    pub time: (u64, u64, u64),
}

impl LogEntry {
    pub fn new(log_type: LogType, message: String) -> Self {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let hours = (now / 3600) % 24;
        let minutes = (now / 60) % 60;
        let seconds = now % 60;

        LogEntry {
            log_type,
            message,
            time: (hours, minutes, seconds),
        }
    }

    pub fn to_html(&self) -> Html<String> {
        let (hours, minutes, seconds) = self.time;
        let class = match self.log_type {
            LogType::Info => "log-info",
            LogType::Error => "log-error",
        };

        Html(format!(
            r#"<div class="log-entry">
                <span class="log-time">[{:02}:{:02}:{:02}]</span>
                <span class="{}">{}</span>
            </div>"#,
            hours, minutes, seconds, class, self.message
        ))
    }
}

pub fn log_error<E: std::fmt::Display>(
    appstate: &AppState,
    error: E,
) -> Html<String> {
    let entry = LogEntry::new(LogType::Error, format!("{}", error));
    let html = entry.to_html();
    let _ = appstate.log_tx.send(html.0.clone());
    html
}

pub fn log_info(
    appstate: &AppState,
    message: impl Into<String>,
) -> Html<String> {
    let entry = LogEntry::new(LogType::Info, message.into());
    let html = entry.to_html();
    let _ = appstate.log_tx.send(html.0.clone());
    html
}
