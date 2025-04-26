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
use config::{create_pin_html, Config};
use futures::{SinkExt, StreamExt};
use gpio::Gpio;
use listenfd::ListenFd;
use logger::{log_error, log_info};
use serde_json::Value;
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
                device: 0,
                actions: Vec::new(),
                gpio_pins: config::default_pins(),
            }
        }
    };

    let (log_tx, _) = broadcast::channel::<String>(100);
    let appstate = AppState {
        gpio: Arc::new(Mutex::new(Gpio::new())),
        actions: Arc::new(Mutex::new(config.actions)),
        stop_it: Arc::new(AtomicBool::new(false)),
        log_tx,
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

async fn get_pins() -> Html<String> {
    let config = match config::load_conf() {
        Ok(conf) => conf,
        Err(_) => {
            println!("failed to load config for gpio pins frontend");
            return Html("<div>Error loading GPIO layout</div>".to_string());
        }
    };

    let mut html = String::new();

    for row in &config.gpio_pins.rows {
        html.push_str("<div class=\"gpio-row\">");

        html.push_str(&create_pin_html(&row.left));

        html.push_str(&create_pin_html(&row.right));

        html.push_str("</div>");
    }

    Html(html)
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

    println!("ws connection opened");

    let (mut sender, mut receiver) = socket.split();

    let mut send_task = tokio::spawn(async move {
        while let Ok(msg) = log_rx.recv().await {
            let fmsg = format!(
                r#"<div id="log-container" hx-swap-oob="afterbegin">{}</div>"#,
                msg
            );
            if sender.send(Message::text(fmsg)).await.is_err() {
                break;
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
    match msg {
        Message::Text(t) => {
            println!("got a text message {}", t);

            if let Ok(json) = serde_json::from_str::<Value>(&t) {
                if let Some(pin_value) = json.get("pin") {
                    let pin_str = match pin_value {
                        Value::String(s) => Some(s.as_str()),
                        _ => None,
                    };

                    if let Some(pin_str) = pin_str {
                        if let Ok(pin_num) = pin_str.parse::<i32>() {
                            println!("Toggling pin: {}", pin_num);

                            let mut gpio = state.gpio.lock().unwrap();
                            match gpio.toggle(pin_num) {
                                Ok(_) => {
                                    let _ = log_info(
                                        &state,
                                        format!("toggled gpio {}", pin_num),
                                    );
                                }
                                Err(e) => {
                                    let _ = log_error(
                                        &state,
                                        format!(
                                            "failed to toggle gpio {}: {}",
                                            pin_num, e
                                        ),
                                    );
                                }
                            }
                        } else {
                            let _ = log_error(
                                &state,
                                format!("invalid GPIO pin {}", pin_str),
                            );
                        }
                    }
                }
            }

            ControlFlow::Continue(())
        }
        Message::Binary(_) => ControlFlow::Continue(()),
        Message::Ping(_) => ControlFlow::Continue(()),
        Message::Pong(_) => ControlFlow::Continue(()),
        Message::Close(close_frame) => {
            if let Some(cf) = close_frame {
                println!(
                    "close with code {} with reason `{}`",
                    cf.code, cf.reason
                );
            } else {
                println!("sent close msg without closeframe");
            }
            ControlFlow::Break(())
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
