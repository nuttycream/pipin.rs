use std::time::{SystemTime, UNIX_EPOCH};

pub enum LogType {
    Info,
    Warning,
    Error,
}

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
}
