mod bindings;
mod errors;
mod logger;

use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        Path, State,
    },
    http::header,
    response::{Html, IntoResponse, Response},
    routing::{any, delete, get, post},
    Form, Router,
};
use bindings::{Gpio, GpioWrapper};
use futures::{SinkExt, StreamExt};
use listenfd::ListenFd;
use logger::{LogEntry, LogType};
use serde::{Deserialize, Serialize};
use std::{
    env,
    error::Error,
    fmt::{self, Display},
    net::SocketAddr,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
    time::Duration,
};
use tokio::{net::TcpListener, sync::broadcast, time::sleep};

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

#[derive(Clone, Serialize, Deserialize, Debug)]
enum Action {
    SetHigh(i32),
    SetLow(i32),
    Delay(i32),
    WaitForHigh(i32),
    WaitForLow(i32),
    SetPullUp(i32),
    SetPullDown(i32),
}

impl Display for Action {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Action::SetHigh(pin) => write!(f, "SETHIGH{pin}"),
            Action::SetLow(pin) => write!(f, "SETLOW{pin}"),
            Action::Delay(time) => write!(f, "DELAY{time}"),
            Action::WaitForHigh(pin) => write!(f, "WAITFORHIGH{pin}"),
            Action::WaitForLow(pin) => write!(f, "WAITFORLOW{pin}"),
            Action::SetPullUp(pin) => write!(f, "SETPULLUP{pin}"),
            Action::SetPullDown(pin) => write!(f, "SETPULLDOWN{pin}"),
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
struct ActionForm {
    action_type: String,
    value: i32,
}

#[derive(Serialize, Deserialize, Debug)]
struct LoopOption {
    should_loop: Option<String>,
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

async fn stop_actions(State(appstate): State<AppState>) {
    println!("attempting to stop");
    appstate.stop_it.store(true, Ordering::Relaxed);
}

async fn start_actions(State(appstate): State<AppState>, Form(input): Form<LoopOption>) {
    println!("starting actions...");
    let should_loop = input.should_loop.as_deref() == Some("true");
    let stop = appstate.stop_it.clone();

    stop.store(false, Ordering::Relaxed);

    loop {
        let actions = appstate.actions.lock().unwrap().clone();

        if actions.is_empty() {
            println!("actions are empty dawg");
            break;
        }

        for i in actions.iter() {
            println!("{i}");
            if stop.load(Ordering::Relaxed) {
                println!("found a stop action");
                break;
            }

            let _ = match i {
                Action::SetHigh(pin) => {
                    let mut gpio = appstate.gpio.lock().unwrap();
                    gpio.set_as_output(*pin).unwrap();
                    gpio.set_high(*pin).unwrap();
                    println!("set high: GPIO {pin}");
                }
                Action::SetLow(pin) => {
                    let mut gpio = appstate.gpio.lock().unwrap();
                    gpio.set_as_output(*pin).unwrap();
                    gpio.set_low(*pin).unwrap();
                    println!("set low: GPIO {pin}");
                }
                Action::Delay(time) => sleep(Duration::from_millis(*time as u64)).await,
                Action::WaitForHigh(pin) => loop {
                    let mut gpio = appstate.gpio.lock().unwrap();
                    if gpio.get_gpio(*pin).unwrap() == true {
                        println!("got HIGH signal: GPIO {pin}");
                        break;
                    }
                    drop(gpio);
                },
                Action::WaitForLow(pin) => loop {
                    let mut gpio = appstate.gpio.lock().unwrap();
                    if gpio.get_gpio(*pin).unwrap() == false {
                        println!("got LOW signal: GPIO {pin}");
                        break;
                    }
                    drop(gpio);
                },
                // gonna use an arbitrary 100 usecond wait_time here
                // not sure if an option should exist later
                Action::SetPullUp(pin) => {
                    let gpio = appstate.gpio.lock().unwrap();
                    gpio.set_pullup(*pin, 100).unwrap();
                    println!("set pullup: GPIO {pin}");
                }
                Action::SetPullDown(pin) => {
                    let gpio = appstate.gpio.lock().unwrap();
                    gpio.set_pulldown(*pin, 100).unwrap();
                    println!("set pulldown: GPIO {pin}");
                }
            };
        }

        if !should_loop {
            break;
        }

        if stop.load(Ordering::Relaxed) {
            println!("stopping here before nexy loop");
            break;
        }
    }
}

async fn delete_action(State(appstate): State<AppState>, Path(index): Path<usize>) {
    let mut actions = appstate.actions.lock().unwrap();
    if index < actions.len() {
        actions.remove(index);
    }
    println!("deleting action");
}

async fn add_action(
    State(appstate): State<AppState>,
    Form(input): Form<ActionForm>,
) -> Html<String> {
    let (action, display_text) = match input.action_type.as_str() {
        // add bounds for adding actions
        // gpio pins should be between 0-27.
        "set-high" => (
            Action::SetHigh(input.value),
            format!("GPIO:{} Set High", input.value),
        ),
        "set-low" => (
            Action::SetLow(input.value),
            format!("GPIO:{} Set Low", input.value),
        ),
        "delay" => (
            Action::Delay(input.value),
            format!("Delay {}ms", input.value),
        ),
        "wait-for-high" => (
            Action::WaitForHigh(input.value),
            format!("Wait For GPIO:{} HIGH", input.value),
        ),
        "wait-for-low" => (
            Action::WaitForLow(input.value),
            format!("Wait For GPIO:{} LOW", input.value),
        ),
        "set-pull-up" => (
            Action::SetPullUp(input.value),
            format!("GPIO:{} Pull-Up", input.value),
        ),
        "set-pull-down" => (
            Action::SetPullDown(input.value),
            format!("GPIO:{} Pull-Down", input.value),
        ),
        _ => return Html(format!("put this in a log somewhere")),
    };

    let mut actions = appstate.actions.lock().unwrap();
    actions.push(action.clone());

    let index = actions.len() - 1;
    drop(actions);

    Html(format!(
        r#"<div class="pin-item" 
        hx-delete="/delete-action/{}" 
        hx-target="closest .pin-item" 
        hx-swap="outerHTML">
            <span class="pin-number">{}</span>
            <span class="pin-delete">DELETE</span>
        </div>"#,
        index, display_text
    ))
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
