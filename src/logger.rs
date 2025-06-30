use std::fmt;
use std::io::{self, Write};
use std::sync::Mutex;
use chrono::Local;
use once_cell::sync::Lazy;

static LOGGER: Lazy<Mutex<()>> = Lazy::new(|| Mutex::new(()));

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum LogLevel {
    Debug,
    Warn,
    Error,
}

impl fmt::Display for LogLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            LogLevel::Debug => "DEBUG",
            LogLevel::Warn => "WARN",
            LogLevel::Error => "ERROR",
        };
        write!(f, "{}", s)
    }
}

impl LogLevel {
    fn color_code(&self) -> &'static str {
        match self {
            LogLevel::Debug => "\x1b[34m", // Blue
            LogLevel::Warn => "\x1b[33m",  // Yellow
            LogLevel::Error => "\x1b[31m", // Red
        }
    }
}

pub struct Logger {
    min_level: LogLevel,
}

impl Logger {
    pub fn new(min_level: LogLevel) -> Self {
        Self { min_level }
    }

    pub fn log(&self, level: LogLevel, msg: &str) {
        if level < self.min_level {
            return;
        }

        let _guard = LOGGER.lock().unwrap();

        let now = Local::now();
        let time_str = now.format("%Y-%m-%d %H:%M:%S");
        let color = level.color_code();
        let reset = "\x1b[0m";

        let output = format!(
            "{} [{}{}{}] {}",
            time_str,
            color,
            level,
            reset,
            msg
        );

        if level == LogLevel::Error {
            let _ = writeln!(&mut io::stderr(), "{}", output);
        } else {
            let _ = writeln!(&mut io::stdout(), "{}", output);
        }
    }

    pub fn debug(&self, msg: &str) {
        self.log(LogLevel::Debug, msg);
    }

    pub fn warn(&self, msg: &str) {
        self.log(LogLevel::Warn, msg);
    }

    pub fn error(&self, msg: &str) {
        self.log(LogLevel::Error, msg);
    }
}
