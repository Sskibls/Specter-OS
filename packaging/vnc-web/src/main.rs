// PhantomKernel OS - Simple Web Terminal Server
use std::net::SocketAddr;
use axum::{routing::get, Router, extract::ws::{WebSocketUpgrade, Message, WebSocket}, response::IntoResponse, extract::State};
use futures_util::{SinkExt, StreamExt};
use tokio::sync::broadcast;
use tower_http::services::ServeDir;

#[derive(Clone)]
struct AppState {
    tx: broadcast::Sender<String>,
}

#[tokio::main]
async fn main() {
    println!("╔═══════════════════════════════════════════════════════════╗");
    println!("║    PhantomKernel OS - Web Terminal v0.1.0                   ║");
    println!("║    Access via: http://localhost:3000                      ║");
    println!("╚═══════════════════════════════════════════════════════════╝");

    let (tx, _rx) = broadcast::channel::<String>(1000);
    let state = AppState { tx: tx.clone() };

    let app = Router::new()
        .route("/ws", get(ws_handler))
        .nest_service("/", ServeDir::new("packaging/vnc-web/static"))
        .with_state(state);

    let addr = SocketAddr::from(([0, 0, 0, 0], 3000));
    println!("🌐 Server listening on http://{}", addr);
    
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

async fn ws_handler(ws: WebSocketUpgrade, State(state): State<AppState>) -> impl IntoResponse {
    ws.on_upgrade(|socket| handle_socket(socket, state))
}

async fn handle_socket(socket: WebSocket, state: AppState) {
    let (mut sender, mut receiver) = socket.split();
    let mut rx = state.tx.subscribe();

    // Send welcome message
    let welcome = r#"{"type":"output","data":"Welcome to PhantomKernel OS Web Terminal\r\n"}"#;
    let _ = sender.send(Message::Text(welcome.to_string())).await;

    // Spawn send task
    let send_task = tokio::spawn(async move {
        while let Ok(msg) = rx.recv().await {
            if sender.send(Message::Text(msg)).await.is_err() {
                break;
            }
        }
    });

    // Receive task
    let tx = state.tx.clone();
    let recv_task = tokio::spawn(async move {
        while let Some(Ok(msg)) = receiver.next().await {
            if let Message::Text(text) = msg {
                if text.contains("\"panic\"") {
                    let _ = tx.send(r#"{"type":"panic","data":"PANIC ACTIVATED"}"#.to_string());
                    println!("⚠️ PANIC");
                }
            }
        }
    });

    tokio::select! {
        _ = send_task => {},
        _ = recv_task => {},
    }
}
