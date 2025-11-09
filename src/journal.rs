use std::{
    borrow::Cow,
    thread::sleep,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use systemd::{journal, JournalRecord, JournalSeek};

use crate::{error, LogAdapter, LogItem, LogLevel};

pub struct JournalAdapter {
    journal: systemd::Journal,
}

impl JournalAdapter {
    pub fn open(options: journal::OpenOptions, seek: JournalSeek) -> Result<Self, std::io::Error> {
        let mut journal = options.open()?;
        journal.seek(seek)?;

        Ok(Self { journal })
    }
}

impl LogAdapter for JournalAdapter {
    fn recv(&mut self) -> Result<LogItem, error::RecvError> {
        loop {
            match self.journal.await_next_entry(None) {
                Ok(Some(entry)) => {
                    return entry_to_log_item(entry).map_err(|e| error::RecvError::Recoverable {
                        context: e,
                        cause: None,
                    })
                }
                Ok(None) => {
                    sleep(Duration::from_millis(10));
                }
                Err(err) => {
                    return Err(error::RecvError::Fatal {
                        context: "failed to read from journal".into(),
                        cause: Some(Box::new(err)),
                    })
                }
            }
        }
    }

    fn try_recv(&mut self) -> Result<LogItem, error::TryRecvError> {
        match self.journal.next_entry() {
            Ok(entry) => {
                if let Some(entry) = entry {
                    entry_to_log_item(entry).map_err(|e| error::TryRecvError::Recoverable {
                        context: e,
                        cause: None,
                    })
                } else {
                    Err(error::TryRecvError::NotReady)
                }
            }
            Err(err) => Err(error::TryRecvError::Fatal {
                context: "failed to read from journal".into(),
                cause: Some(Box::new(err)),
            }),
        }
    }
}

fn entry_to_log_item(entry: JournalRecord) -> Result<LogItem, Cow<'static, str>> {
    log::debug!("journald entry: {:?}", entry);

    let mut hostname = None;
    let mut unit = None;
    let mut message = None;
    let mut timestamp = None;
    let mut priority = None;
    for (k, v) in entry.into_iter() {
        match k.as_str() {
            "MESSAGE" => message = Some(v),
            "PRIORITY" => {
                priority = Some(
                    *systemd_priority_rankings()
                        .get(
                            v.parse::<usize>()
                                .map_err(|_| format!("failed to parse priority: {}", v))?,
                        )
                        .ok_or(format!("invalid journal entry priority: {}", v))?,
                );
            }
            "_SYSTEMD_UNIT" => unit = Some(v),
            "_SOURCE_REALTIME_TIMESTAMP" => {
                let timestamp_usec: u64 = v
                    .parse()
                    .map_err(|_| format!("failed to parse timestamp: {}", v))?;
                timestamp = Some(
                    UNIX_EPOCH
                        .checked_add(Duration::from_micros(timestamp_usec))
                        .ok_or("time overflow")?,
                );
            }
            "_HOSTNAME" => {
                hostname = Some(v);
            }
            _ => (),
        }
    }

    Ok(LogItem {
        hostname: hostname.ok_or("missing hostname field from jounral entry")?,
        unit: unit.unwrap_or_else(|| "unknown".to_owned()),
        level: priority.ok_or("missing priority field from journal entry")?,
        message: message.ok_or("missing message field from journal entry")?,
        timestamp: timestamp.unwrap_or(SystemTime::now()),
    })
}

const fn systemd_priority_rankings() -> &'static [LogLevel] {
    &[
        LogLevel::Emergency,
        LogLevel::Alert,
        LogLevel::Critical,
        LogLevel::Error,
        LogLevel::Warning,
        LogLevel::Notice,
        LogLevel::Info,
        LogLevel::Debug,
    ]
}
