mod actions;
mod config;
mod errors;
mod gpio;
mod logger;

use actions::{
    add_action, delete_action, get_actions, start_actions, stop_actions, Action,
};
use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        State,
    },
    http::header,
    response::{Html, IntoResponse, Response},
    routing::{any, delete, get, post},
    Router,
};
use config::Config;
use futures::{SinkExt, StreamExt};
use gpio::{Gpio, PinLevel};
use listenfd::ListenFd;
use logger::{log_error, log_info};
use std::{
    env,
    error::Error,
    net::SocketAddr,
    ops::ControlFlow,
    sync::{atomic::AtomicBool, Arc, Mutex},
};
use tokio::{net::TcpListener, sync::broadcast};

#[derive(Clone)]
struct AppState {
    gpio: Arc<Mutex<Gpio>>,
    actions: Arc<Mutex<Vec<Action>>>,
    stop_it: Arc<AtomicBool>,
    log_tx: broadcast::Sender<String>,
    toggle_tx: broadcast::Sender<String>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let port = env::args()
        .nth(1)
        .and_then(|arg| arg.parse::<i32>().ok())
        .unwrap_or(3000);
    let addr = format!("0.0.0.0:{port}");

    let config = match config::load_conf() {
        Ok(conf) => conf,
        Err(_) => {
            println!("failed to load config");
            Config {
                actions: Vec::new(),
            }
        }
    };

    let (log_tx, _) = broadcast::channel::<String>(100);
    let (toggle_tx, _) = broadcast::channel::<String>(100);
    let appstate = AppState {
        gpio: Arc::new(Mutex::new(Gpio::new())),
        actions: Arc::new(Mutex::new(config.actions)),
        stop_it: Arc::new(AtomicBool::new(false)),
        log_tx,
        toggle_tx,
    };

    let app = Router::new()
        .route("/", get(serve_html))
        .route("/htmx.min.js", get(serve_js))
        .route("/style.css", get(serve_css))
        .route("/setup", get(setup))
        .route("/reset", get(reset))
        .route("/terminate", get(terminate))
        .route("/get-pins", get(get_pins))
        .route("/add-action", post(add_action))
        .route("/delete-action/{index}", delete(delete_action))
        .route("/start-actions", post(start_actions))
        .route("/stop-actions", post(stop_actions))
        .route("/get-actions", get(get_actions))
        .route("/ws", any(handle_websocket))
        .with_state(appstate);

    let mut listenfd = ListenFd::from_env();
    let listener = match listenfd.take_tcp_listener(0)? {
        Some(listener) => {
            listener.set_nonblocking(true)?;
            TcpListener::from_std(listener)?
        }
        None => TcpListener::bind(addr).await?,
    };

    let shutdown = async {
        tokio::signal::ctrl_c()
            .await
            .expect("failed to get ctrl+c signhandle");
    };

    println!("listening on {}", listener.local_addr()?);
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .with_graceful_shutdown(shutdown)
    .await?;

    Ok(())
}

async fn get_pins(State(appstate): State<AppState>) -> Html<String> {
    let gpio = match appstate.gpio.lock() {
        Ok(gpio) => gpio,
        Err(_) => {
            println!("failed to load config for gpio pins frontend");
            return Html("<div>Error loading GPIO layout</div>".to_string());
        }
    };

    match gpio.get_html_pins() {
        Ok(html) => Html(html),
        Err(e) => {
            println!("{e}");
            log_error(&appstate, "Failed to get pins")
        }
    }
}

async fn setup(State(appstate): State<AppState>) -> impl IntoResponse {
    let mut gpio = appstate.gpio.lock().unwrap();

    match gpio.setup() {
        Ok(_) => log_info(&appstate, "GPIO initialized"),
        Err(e) => {
            println!("{e}");
            log_error(&appstate, format!("failed to initialize gpio: {e}"))
        }
    }
}

async fn reset(State(appstate): State<AppState>) -> impl IntoResponse {
    let mut gpio = appstate.gpio.lock().unwrap();

    match gpio.reset() {
        Ok(_) => log_info(&appstate, "GPIO reset"),
        Err(e) => {
            println!("{e}");
            log_error(&appstate, format!("failed to reset gpio: {e}"))
        }
    }
}

async fn terminate(State(appstate): State<AppState>) -> impl IntoResponse {
    let mut gpio = appstate.gpio.lock().unwrap();

    match gpio.terminate() {
        Ok(_) => log_info(&appstate, "GPIO terminated"),
        Err(e) => {
            println!("{e}");
            log_error(&appstate, format!("failed to terminate gpio: {e}"))
        }
    }
}

async fn handle_websocket(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
) -> Response {
    ws.on_upgrade(move |socket| handle_socket(socket, state))
}

async fn handle_socket(socket: WebSocket, state: AppState) {
    let mut log_rx = state.log_tx.subscribe();
    let mut toggle_rx = state.toggle_tx.subscribe();

    println!("ws connection opened");

    let (mut sender, mut receiver) = socket.split();

    let mut send_task = tokio::spawn(async move {
        // separating tasks for logging
        // and toggling should be relatively good enough
        loop {
            tokio::select! {
                //logging
                result = log_rx.recv() => {
                    if let Ok(msg) = result {
                        let fmsg = format!(
                            r#"<div id="log-container" hx-swap-oob="afterbegin">{}</div>"#,
                            msg
                        );
                        if sender.send(Message::text(fmsg)).await.is_err() {
                            break;
                        }
                    }
                }

                //toggling
                result = toggle_rx.recv() => {
                    if let Ok(msg) = result {
                        if sender.send(Message::text(msg)).await.is_err() {
                            break;
                        }
                    }
                }
            }
        }
    });

    let mut recv_task = tokio::spawn(async move {
        while let Some(Ok(msg)) = receiver.next().await {
            if process_message(msg, state.clone()).is_break() {
                break;
            }
        }
    });

    tokio::select! {
        rv_a = (&mut send_task) => {
            match rv_a {
                Ok(_) => println!("messages sent"),
                Err(a) => println!("Error sending messages {a:?}")
            }
            recv_task.abort();
        },
        rv_b = (&mut recv_task) => {
            match rv_b {
                Ok(_) => println!("received messages"),
                Err(b) => println!("Error receiving messages {b:?}")
            }
            send_task.abort();
        }
    }

    println!("ws connection closed");
}

fn process_message(msg: Message, state: AppState) -> ControlFlow<(), ()> {
    if let Message::Close(close_frame) = &msg {
        if let Some(cf) = close_frame {
            println!("close with code {} with reason `{}`", cf.code, cf.reason);
        } else {
            println!("sent close msg without closeframe");
        }
        return ControlFlow::Break(());
    }

    if let Message::Text(t) = msg {
        if let Some(pin) = serde_json::from_str::<serde_json::Value>(&t)
            .ok()
            .and_then(|json| {
                json.get("pin")
                    .and_then(|val| val.as_str())
                    .and_then(|s| s.parse::<i32>().ok())
            })
        {
            toggle_pin(pin, state)
        }
    }

    ControlFlow::Continue(())
}

fn toggle_pin(pin: i32, state: AppState) {
    println!("Toggling pin: {}", pin);

    let mut gpio = match state.gpio.lock() {
        Ok(guard) => guard,
        Err(e) => {
            println!("Failed to get GPIO lock: {}", e);
            return;
        }
    };

    // todo this needs to be redone as well
    // as update_pin since we're reacquiring the lock
    match gpio.toggle(pin) {
        Ok(new_level) => {
            let level_str = match new_level {
                PinLevel::High => "HIGH",
                PinLevel::Low => "LOW",
            };

            let _ = log_info(
                &state,
                format!("Toggle GPIO {} -> {}", pin, level_str),
            );

            let _ = state.toggle_tx.send(gpio.update_pin(pin));
        }
        Err(e) => {
            let _ = log_error(
                &state,
                format!("failed to toggle gpio {}: {}", pin, e),
            );
        }
    }
}

async fn serve_html() -> Html<&'static str> {
    let html = include_str!("../frontend/index.html");
    Html(html)
}

async fn serve_js() -> impl IntoResponse {
    let js = include_str!("../frontend/htmx.min.js");
    ([(header::CONTENT_TYPE, "application/javascript")], js)
}

async fn serve_css() -> impl IntoResponse {
    let css = include_str!("../frontend/style.css");
    ([(header::CONTENT_TYPE, "text/css")], css)
}
