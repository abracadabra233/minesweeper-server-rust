pub mod logic;
pub mod utils;
use crate::logic::game::ws_handler;
use axum::{routing::get, Router};
use clap::Parser;
use colored::*;
use env_logger;
use env_logger::Builder;
use log::LevelFilter;
use std::io::Write;
use tokio;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// 监听的主机地址
    #[arg(long, default_value = "127.0.0.1")]
    host: String,

    /// 监听的端口
    #[arg(long, default_value = "15436")]
    port: u16,
}

#[tokio::main]
async fn main() {
    // 解析命令行参数
    let args = Args::parse();
    let addr = format!("{}:{}", args.host, args.port);

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
    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
