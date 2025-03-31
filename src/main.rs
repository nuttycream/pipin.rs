mod bindings;
mod errors;
mod logger;

use axum::{
    extract::{Path, State},
    http::header,
    response::{Html, IntoResponse},
    routing::{delete, get, post},
    Form, Router,
};
use bindings::{Gpio, GpioWrapper};
use fastwebsockets::{upgrade, OpCode, WebSocketError};
use listenfd::ListenFd;
use serde::Deserialize;
//use logger::LogType;
use std::{
    env,
    error::Error,
    fmt::{format, Display},
    net::SocketAddr,
    sync::{Arc, Mutex},
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use tokio::{net::TcpListener, time::sleep};

#[derive(Clone)]
enum Action {
    SetHigh(i32),
    SetLow(i32),
    Delay(i32),
    WaitFor(i32),
}

impl Display for Action {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Action::SetHigh(pin) => write!(f, "SETHIGH{pin}"),
            Action::SetLow(pin) => write!(f, "SETLOW{pin}"),
            Action::Delay(time) => write!(f, "DELAY{time}"),
            Action::WaitFor(pin) => write!(f, "WAITFOR{pin}"),
        }
    }
}

#[derive(Deserialize)]
struct ActionForm {
    action_type: String,
    value: i32,
}

#[derive(Clone)]
struct AppState {
    gpio: Arc<Mutex<Gpio>>,
    actions: Arc<Mutex<Vec<Action>>>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let port = env::args()
        .nth(1)
        .and_then(|arg| arg.parse::<i32>().ok())
        .unwrap_or(3000);
    let addr = format!("0.0.0.0:{port}");

    let appstate = AppState {
        gpio: Arc::new(Mutex::new(Gpio::new())),
        actions: Arc::new(Mutex::new(Vec::new())),
    };

    let app = Router::new()
        .route("/", get(serve_html))
        .route("/htmx.min.js", get(serve_js))
        .route("/style.css", get(serve_css))
        .route("/ws", get(ws_handler))
        .route("/setup", get(setup))
        .route("/reset", get(reset))
        .route("/terminate", get(terminate))
        .route("/{pin}", get(toggle))
        .route("/add-action", post(add_action))
        .route("/delete-action/{index}", delete(delete_action))
        .route("/start-actions", post(start_actions))
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

async fn start_actions(State(appstate): State<AppState>) {
    println!("starting actions...");
    let actions = appstate.actions.lock().unwrap().clone();
    for i in actions.iter() {
        println!("{i}");
        // FUCK ERROR HANDLING UNWRAPPP EVERYTHHHINNG
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
            Action::WaitFor(pin) => loop {
                let mut gpio = appstate.gpio.lock().unwrap();
                gpio.set_as_input(*pin).unwrap();
                if gpio.get_gpio(*pin).unwrap() {
                    println!("got signal: GPIO {pin}");
                    break;
                }
                drop(gpio);
            },
        };
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
        "wait-for" => (
            Action::WaitFor(input.value),
            format!("Wait For GPIO:{}", input.value),
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

    // refactor this out bum
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let hours = (now / 3600) % 24;
    let minutes = (now / 60) % 60;
    let seconds = now % 60;

    let gpio_pin = match pin.parse::<i32>() {
        Ok(num) => num,
        Err(_) => {
            return Html(format!(
                r#"<div class="log-entry">
                    <span class="log-time">[{:02}:{:02}:{:02}]</span>
                    <span class="log-error">invalid GPIO pin {}</span>
                </div>"#,
                hours, minutes, seconds, pin
            ));
        }
    };

    let res: String = match gpio.toggle(gpio_pin) {
        Ok(_) => format!("toggled gpio {}", gpio_pin),
        Err(e) => {
            println!("{e}");
            format!("failed to toggle gpio {e}").to_string()
        }
    };

    let log_entry = format!(
        r#"<div class="log-entry">
            <span class="log-time">[{:02}:{:02}:{:02}]</span>
            <span class="log-info">{}</span>
        </div>"#,
        hours, minutes, seconds, res
    );

    Html(log_entry)
}

async fn setup(State(appstate): State<AppState>) -> impl IntoResponse {
    let mut gpio = appstate.gpio.lock().unwrap();
    let res: String = match gpio.setup() {
        Ok(_) => "GPIO initialized".to_string(),
        Err(e) => {
            println!("{e}");
            format!("failed to initialize gpio {e}")
        }
    };

    // extract this to a helper func later on
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let hours = (now / 3600) % 24;
    let minutes = (now / 60) % 60;
    let seconds = now % 60;

    let log_entry = format!(
        r#"<div class="log-entry">
            <span class="log-time">[{:02}:{:02}:{:02}]</span>
            <span class="log-info">{}</span>
        </div>"#,
        hours, minutes, seconds, res
    );

    Html(log_entry)
}

async fn reset(State(appstate): State<AppState>) -> impl IntoResponse {
    let mut gpio = appstate.gpio.lock().unwrap();
    let res: String = match gpio.reset() {
        Ok(_) => "GPIO reset".to_string(),
        Err(e) => {
            println!("{e}");
            format!("failed to reset gpio {e}")
        }
    };

    // extract this to a helper func later on
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let hours = (now / 3600) % 24;
    let minutes = (now / 60) % 60;
    let seconds = now % 60;

    let log_entry = format!(
        r#"<div class="log-entry">
            <span class="log-time">[{:02}:{:02}:{:02}]</span>
            <span class="log-info">{}</span>
        </div>"#,
        hours, minutes, seconds, res
    );

    Html(log_entry)
}

async fn terminate(State(appstate): State<AppState>) -> impl IntoResponse {
    let mut gpio = appstate.gpio.lock().unwrap();
    let res: String = match gpio.terminate() {
        Ok(_) => "GPIO terminated".to_string(),
        Err(e) => {
            println!("{e}");
            format!("failed to terminate gpio {e}")
        }
    };

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let hours = (now / 3600) % 24;
    let minutes = (now / 60) % 60;
    let seconds = now % 60;

    let log_entry = format!(
        r#"<div class="log-entry">
            <span class="log-time">[{:02}:{:02}:{:02}]</span>
            <span class="log-info">{}</span>
        </div>"#,
        hours, minutes, seconds, res
    );

    Html(log_entry)
}

async fn ws_handler(ws: upgrade::IncomingUpgrade) -> impl IntoResponse {
    let (response, fut) = ws.upgrade().unwrap();

    tokio::task::spawn(async move {
        if let Err(e) = handle_client(fut).await {
            eprintln!("Error in websocket connection: {}", e);
        }
    });

    response
}

async fn handle_client(fut: upgrade::UpgradeFut) -> Result<(), WebSocketError> {
    let mut ws = fastwebsockets::FragmentCollector::new(fut.await?);

    loop {
        let frame = ws.read_frame().await?;
        match frame.opcode {
            OpCode::Close => break,
            OpCode::Text | OpCode::Binary => {
                ws.write_frame(frame).await?;
            }
            _ => {}
        }
    }

    Ok(())
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
