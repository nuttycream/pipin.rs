mod actions;
mod bindings;
mod errors;
mod logger;

use actions::{
    add_action,
    delete_action,
    start_actions,
    stop_actions,
    Action,
};
use axum::{
    extract::{
        ws::{
            Message,
            WebSocket,
            WebSocketUpgrade,
        },
        Path,
        State,
    },
    http::header,
    response::{
        Html,
        IntoResponse,
        Response,
    },
    routing::{
        any,
        delete,
        get,
        post,
    },
    Router,
};
use bindings::{
    Gpio,
    GpioWrapper,
};
use futures::{
    SinkExt,
    StreamExt,
};
use listenfd::ListenFd;
use logger::{
    LogEntry,
    LogType,
};
use std::{
    env,
    error::Error,
    net::SocketAddr,
    sync::{
        atomic::AtomicBool,
        Arc,
        Mutex,
    },
};
use tokio::{
    net::TcpListener,
    sync::broadcast,
};

fn log_error<E: std::fmt::Display>(appstate: &AppState, error: E) -> Html<String> {
    let entry = LogEntry::new(LogType::Error, format!("{}", error));
    let html = entry.to_html();
    let _ = appstate.log_tx.send(html.0.clone());
    html
}

fn log_info(appstate: &AppState, message: impl Into<String>) -> Html<String> {
    let entry = LogEntry::new(LogType::Info, message.into());
    let html = entry.to_html();
    let _ = appstate.log_tx.send(html.0.clone());
    html
}

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

    let (log_tx, _) = broadcast::channel::<String>(100);
    let appstate = AppState {
        gpio: Arc::new(Mutex::new(Gpio::new())),
        actions: Arc::new(Mutex::new(Vec::new())),
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
        .route("/{pin}", get(toggle))
        .route("/add-action", post(add_action))
        .route("/delete-action/{index}", delete(delete_action))
        .route("/start-actions", post(start_actions))
        .route("/stop-actions", post(stop_actions))
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

async fn toggle(State(appstate): State<AppState>, Path(pin): Path<String>) -> impl IntoResponse {
    let mut gpio = appstate.gpio.lock().unwrap();

    let gpio_pin = match pin.parse::<i32>() {
        Ok(num) => num,
        Err(_) => {
            return log_error(&appstate, format!("invalid GPIO pin {}", pin));
        }
    };

    match gpio.toggle(gpio_pin) {
        Ok(_) => log_info(&appstate, format!("toggled gpio {}", gpio_pin)),
        Err(e) => {
            println!("{e}");
            log_error(&appstate, format!("failed to toggle gpio: {e}"))
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
async fn handle_websocket(ws: WebSocketUpgrade, State(state): State<AppState>) -> Response {
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
            println!("sending websocket message: {}", fmsg);
            if sender.send(Message::text(fmsg)).await.is_err() {
                break;
            }
        }
    });

    let mut recv_task = tokio::spawn(async move {
        while let Some(result) = receiver.next().await {
            match result {
                Ok(_) => {}
                Err(_) => break,
            }
        }
    });

    tokio::select! {
        _ = &mut send_task => recv_task.abort(),
        _ = &mut recv_task => send_task.abort(),
    }

    println!("ws connection closed");
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
