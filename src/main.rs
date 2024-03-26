pub mod logic;
pub mod utils;
use crate::logic::game::ws_handler;
use axum::{routing::get, Router};
use colored::*;
use env_logger;
use env_logger::Builder;
use log::LevelFilter;
use std::io::Write;
use tokio;

#[tokio::main]
async fn main() {
    // 初始化日志系统
    Builder::new()
        .filter_level(LevelFilter::Info)
        .format(|buf, record| {
            let level = match record.level() {
                log::Level::Error => record.level().to_string().red(),
                log::Level::Warn => record.level().to_string().yellow(),
                log::Level::Info => record.level().to_string().green(),
                log::Level::Debug => record.level().to_string().blue(),
                log::Level::Trace => record.level().to_string().purple(),
            };

            writeln!(
                buf,
                "[{}:{}][{}] - {}",
                record.file().unwrap_or("unknown"),
                record.line().unwrap_or(0),
                level,
                record.args()
            )
        })
        .init();

    let app = Router::new().route("/ws", get(ws_handler));
    let listener = tokio::net::TcpListener::bind("0.0.0.0:15437").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
