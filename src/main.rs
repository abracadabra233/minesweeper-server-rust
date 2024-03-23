pub mod logic;
pub mod utils;

use crate::logic::game::handle_socket;
use axum::extract::WebSocketUpgrade;
use axum::response::Response;
use axum::{routing::get, Router};
use env_logger;
use env_logger::Builder;
use log::LevelFilter;
use std::io::Write;
use tokio;

pub async fn ws_handler(ws: WebSocketUpgrade) -> Response {
    ws.on_upgrade(handle_socket)
}

#[tokio::main]
async fn main() {
    // 初始化日志系统
    Builder::new()
        .filter_level(LevelFilter::Info)
        .format(|buf, record| {
            writeln!(
                buf,
                "[{}:{}][{}] - {}",
                record.file().unwrap_or("unknown"),
                record.line().unwrap_or(0),
                record.level(),
                record.args()
            )
        })
        .init();

    let app = Router::new().route("/ws", get(ws_handler));
    let listener = tokio::net::TcpListener::bind("0.0.0.0:15437").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
