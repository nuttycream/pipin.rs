mod bindings;
mod errors;
mod logger;

use axum::{
    extract::{
        Path,
        State,
    },
    http::header,
    response::{
        Html,
        IntoResponse,
    },
    routing::get,
    Router,
};
use bindings::{
    Gpio,
    GpioController,
};
use fastwebsockets::{
    upgrade,
    OpCode,
    WebSocketError,
};
use listenfd::ListenFd;
use logger::LogType;
use std::{
    env,
    error::Error,
    net::SocketAddr,
    sync::{
        Arc,
        Mutex,
    },
    time::{
        SystemTime,
        UNIX_EPOCH,
    },
};
use tokio::net::TcpListener;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let port = env::args()
        .nth(1)
        .and_then(|arg| arg.parse::<i32>().ok())
        .unwrap_or(3000);
    let addr = format!("0.0.0.0:{port}");

    let gpio = Arc::new(Mutex::new(Gpio::new()));

    let app = Router::new()
        .route("/", get(serve_html))
        .route("/htmx.min.js", get(serve_js))
        .route("/style.css", get(serve_css))
        .route("/ws", get(ws_handler))
        .route("/setup", get(setup))
        .route("/reset", get(reset))
        .route("/terminate", get(terminate))
        .route("/{pin}", get(toggle))
        .with_state(gpio);

    let mut listenfd = ListenFd::from_env();
    let listener = match listenfd.take_tcp_listener(0)? {
        Some(listener) => {
            listener.set_nonblocking(true)?;
            TcpListener::from_std(listener)?
        }
        None => TcpListener::bind(addr).await?,
    };

    println!("listening on {}", listener.local_addr()?);
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await?;

    Ok(())
}

async fn toggle(
    State(gpio): State<Arc<Mutex<Gpio>>>,
    Path(pin): Path<String>,
) -> impl IntoResponse {
    let mut gpio = gpio.lock().unwrap();

    // refactor this out bum
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let hours = (now / 3600) % 24;
    let minutes = (now / 60) % 60;
    let seconds = now % 60;

    if !gpio.is_initialized() {
        return Html(format!(
            r#"<div class="log-entry">
                    <span class="log-time">[{:02}:{:02}:{:02}]</span>
                    <span class="log-error">gpio not initialized/setup</span>
                </div>"#,
            hours, minutes, seconds
        ));
    }

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
            "failed to toggle gpio".to_string()
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

async fn setup(State(gpio): State<Arc<Mutex<Gpio>>>) -> impl IntoResponse {
    let mut gpio = gpio.lock().unwrap();
    let res = match gpio.setup() {
        Ok(_) => "GPIO initialized",
        Err(e) => {
            println!("{e}");
            "Failed to initialize gpio"
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

async fn reset(State(gpio): State<Arc<Mutex<Gpio>>>) -> impl IntoResponse {
    let mut gpio = gpio.lock().unwrap();
    let res = match gpio.reset() {
        Ok(_) => "GPIO reset",
        Err(e) => {
            println!("{e}");
            "Failed to reset gpio"
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

async fn terminate(State(gpio): State<Arc<Mutex<Gpio>>>) -> impl IntoResponse {
    let mut gpio = gpio.lock().unwrap();
    let res = match gpio.terminate() {
        Ok(_) => "GPIO terminated",
        Err(e) => {
            println!("{e}");
            "Failed to terminate gpio"
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
