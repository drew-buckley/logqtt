use signal_hook::{consts::SIGINT, consts::SIGTERM, iterator::Signals};
use std::{
    sync::{atomic::AtomicBool, Arc},
    thread::sleep,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use argh::FromArgs;
use logqtt::{client::LogqttClient, journal::JournalAdapter, LogAdapter};
use rumqttc::{Connection, ConnectionError, MqttOptions};
use systemd::{journal::OpenOptions, JournalSeek};

/// journald to MQTT
#[derive(FromArgs)]
struct Args {
    /// broker host (default: 127.0.0.1)
    #[argh(
        option,
        short = 'h',
        long = "host",
        default = "String::from(\"127.0.0.1\")"
    )]
    host: String,

    /// broker port (default: 1883)
    #[argh(option, short = 'p', long = "port", default = "1883")]
    port: u16,

    /// base topic (default: logqttv1)
    #[argh(
        option,
        short = 't',
        long = "base-topic",
        default = "String::from(\"logqttv1\")"
    )]
    base_topic: String,

    /// MQTT ID (default: hostname)
    #[argh(option, long = "id", default = "hostname()")]
    id: String,

    /// use syslog formatting for log
    #[argh(switch, long = "syslog")]
    syslog: bool,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Args = argh::from_env();
    init_logging(args.syslog);
    log::info!("ID: {}", args.id);
    log::info!("Broker: {}:{}", args.host, args.port);
    let (client, connection) =
        rumqttc::Client::new(MqttOptions::new(args.id, args.host, args.port), 32);
    let mut client = LogqttClient::new(client, args.base_topic);

    let mut signals = Signals::new([SIGINT, SIGTERM])?;
    let mut journal = JournalAdapter::open(open_options(), seek_now())?;
    let should_run = Arc::new(AtomicBool::new(true));

    let should_run_clone = should_run.clone();
    std::thread::spawn(move || {
        let _ = signals.forever().next();
        log::warn!("Close signaled");
        should_run_clone.store(false, std::sync::atomic::Ordering::Relaxed);
    });

    let should_run_clone = should_run.clone();
    let conn_loop_handle = std::thread::spawn(|| run_connection_loop(connection, should_run_clone));
    while should_run.load(std::sync::atomic::Ordering::Relaxed) {
        if conn_loop_handle.is_finished() {
            break;
        }

        match journal.try_recv() {
            Ok(log_item) => {
                client.push(log_item)?;
            }
            Err(err) => match &err {
                logqtt::error::TryRecvError::NotReady => sleep(Duration::from_millis(100)),
                logqtt::error::TryRecvError::Recoverable { .. } => {
                    log::error!("{err}")
                }
                _ => return Err(err.into()),
            },
        }
    }

    log::info!("Closing");

    Ok(())
}

fn run_connection_loop(
    mut connection: Connection,
    should_run: Arc<AtomicBool>,
) -> Result<(), ConnectionError> {
    for notification in connection.iter() {
        let event = notification?;
        log::debug!("{:?}", event);

        if !should_run.load(std::sync::atomic::Ordering::Relaxed) {
            break;
        }
    }

    log::info!("Connection loop done");

    Ok(())
}

fn hostname() -> String {
    whoami::fallible::hostname().expect("failed to get hostname")
}

fn seek_now() -> JournalSeek {
    JournalSeek::ClockRealtime {
        usec: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_micros()
            .try_into()
            .unwrap(),
    }
}

fn open_options() -> OpenOptions {
    let mut open_options = OpenOptions::default();
    open_options.all_namespaces(true);
    open_options
}

fn init_logging(use_syslog: bool) {
    use std::io::Write;

    let mut log_builder =
        env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"));

    if use_syslog {
        log_builder.format(|buffer, record| {
            writeln!(buffer, "<{}>{}", record.level() as u8 + 2, record.args())
        });
    }
    log_builder.init();
}
