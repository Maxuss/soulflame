pub mod net_io;
pub mod network;
pub mod util;
#[cfg(test)]
pub mod tests;

use std::path::Path;

use log::{info, LevelFilter};
use log4rs::append::console::ConsoleAppender;

use log4rs::append::rolling_file::policy::compound::roll::fixed_window::FixedWindowRoller;
use log4rs::append::rolling_file::policy::compound::trigger::size::SizeTrigger;
use log4rs::append::rolling_file::policy::compound::CompoundPolicy;
use log4rs::append::rolling_file::RollingFileAppender;
use log4rs::config::{Appender, Logger, Root};
use log4rs::encode::pattern::PatternEncoder;
use log4rs::{init_config, Config};

#[tokio::main]
async fn main() {
    configure_logging().await;

    info!("Starting SoulFlame server...");

    info!("Closing server...");
}

async fn configure_logging() {
    let path = Path::new("./logs/latest.log");
    if path.exists() {}

    let pattern = "[{d(%Y-%m-%d %H:%M:%S)}] <{M}> {h([{l}])}: {m}\n";
    let stdout = ConsoleAppender::builder()
        .encoder(Box::new(PatternEncoder::new(pattern)))
        .build();

    let logfile = RollingFileAppender::builder()
        .encoder(Box::new(PatternEncoder::new(pattern)))
        .build(
            "logs/latest.log",
            Box::new(CompoundPolicy::new(
                Box::new(SizeTrigger::new(4 * 1024)),
                Box::new(
                    FixedWindowRoller::builder()
                        .build("logs/log_{}.old.gz", 4)
                        .expect("Could not initialize logger roller."),
                ),
            )),
        )
        .expect("Could not initialize file logging");

    let config = Config::builder()
        .appender(Appender::builder().build("stdout", Box::new(stdout)))
        .appender(Appender::builder().build("logfile", Box::new(logfile)))
        .logger(Logger::builder().build("soulflame::general", LevelFilter::Info))
        .build(
            Root::builder()
                .appender("stdout")
                .appender("logfile")
                .build(LevelFilter::Info),
        )
        .expect("Could not build logger config");

    init_config(config).expect("Could not initialize logger config");
}
