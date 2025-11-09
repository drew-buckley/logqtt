use std::time::SystemTime;

pub mod client;

#[cfg(feature = "journal-adapter")]
pub mod journal;

pub trait LogAdapter {
    fn recv(&mut self) -> Result<LogItem, error::RecvError>;
    fn try_recv(&mut self) -> Result<LogItem, error::TryRecvError>;
}

#[derive(Clone, Debug)]
pub struct LogItem {
    pub hostname: String,
    pub unit: String,
    pub timestamp: SystemTime,
    pub level: LogLevel,
    pub message: String,
}

#[derive(Clone, Copy, Debug)]
pub enum LogLevel {
    Emergency,
    Alert,
    Critical,
    Error,
    Warning,
    Notice,
    Info,
    Debug,
}

impl AsRef<str> for LogLevel {
    fn as_ref(&self) -> &str {
        match self {
            LogLevel::Emergency => "emergency",
            LogLevel::Alert => "alert",
            LogLevel::Critical => "critical",
            LogLevel::Error => "error",
            LogLevel::Warning => "warning",
            LogLevel::Notice => "notice",
            LogLevel::Info => "info",
            LogLevel::Debug => "debug",
        }
    }
}

pub mod error {
    use std::{borrow::Cow, fmt::Display};

    #[derive(Debug)]
    pub enum RecvError {
        Closed,
        Recoverable {
            context: Cow<'static, str>,
            cause: Option<Box<dyn std::error::Error>>,
        },
        Fatal {
            context: Cow<'static, str>,
            cause: Option<Box<dyn std::error::Error>>,
        },
    }

    impl std::error::Error for RecvError {}

    impl Display for RecvError {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self {
                RecvError::Closed => write!(f, "log connection closed"),
                RecvError::Recoverable { context, cause } => {
                    write!(
                        f,
                        "recoverable error: {}{}",
                        context,
                        cause
                            .as_ref()
                            .map(|e| format!("; caused by {}", e))
                            .unwrap_or_else(|| "".to_owned())
                    )
                }
                RecvError::Fatal { context, cause } => {
                    write!(
                        f,
                        "fatal error: {}{}",
                        context,
                        cause
                            .as_ref()
                            .map(|e| format!("; caused by {}", e))
                            .unwrap_or_else(|| "".to_owned())
                    )
                }
            }
        }
    }

    #[derive(Debug)]
    pub enum TryRecvError {
        NotReady,
        Closed,
        Recoverable {
            context: Cow<'static, str>,
            cause: Option<Box<dyn std::error::Error>>,
        },
        Fatal {
            context: Cow<'static, str>,
            cause: Option<Box<dyn std::error::Error>>,
        },
    }

    impl std::error::Error for TryRecvError {}

    impl Display for TryRecvError {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self {
                TryRecvError::Closed => write!(f, "log connection closed"),
                TryRecvError::NotReady => write!(f, "log not ready"),
                TryRecvError::Recoverable { context, cause } => {
                    write!(
                        f,
                        "recoverable error: {}{}",
                        context,
                        cause
                            .as_ref()
                            .map(|e| format!("; caused by {}", e))
                            .unwrap_or_else(|| "".to_owned())
                    )
                }
                TryRecvError::Fatal { context, cause } => {
                    write!(
                        f,
                        "fatal error: {}{}",
                        context,
                        cause
                            .as_ref()
                            .map(|e| format!("; caused by {}", e))
                            .unwrap_or_else(|| "".to_owned())
                    )
                }
            }
        }
    }
}
