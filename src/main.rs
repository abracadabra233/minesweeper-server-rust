pub mod logic;
pub mod utils;

use crate::logic::game::ws_handler;
use axum::{routing::get, Router};
use env_logger;
use env_logger::Builder;
use log::LevelFilter;
use tokio;

#[tokio::main]
async fn main() {
    // 初始化日志系统
    Builder::new()
        .filter_level(LevelFilter::Debug) // 设置全局日志级别为Info
        .init();

    let app = Router::new().route("/ws", get(ws_handler));
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
