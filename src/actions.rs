use crate::{
    config::save_actions,
    gpio::{PinLevel, PullType},
    logger::{log_error, log_info},
    AppState,
};

use axum::{
    extract::{Path, State},
    response::Html,
    Form,
};
use serde::{Deserialize, Serialize};
use std::{
    fmt::{self, Display},
    sync::atomic::Ordering,
    time::Duration,
};
use tokio::time::sleep;

#[derive(Clone, Serialize, Deserialize, Debug)]
pub enum Action {
    SetHigh(i32),
    SetLow(i32),
    Delay(i32),
    WaitForHigh(i32),
    WaitForLow(i32),
    SetPullUp(i32),
    SetPullDown(i32),
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ActionForm {
    pub action_type: String,
    pub value: i32,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct LoopOption {
    pub should_loop: Option<String>,
}

impl Display for Action {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Action::SetHigh(pin) => {
                write!(f, "SETHIGH{pin}")
            }
            Action::SetLow(pin) => write!(f, "SETLOW{pin}"),
            Action::Delay(time) => write!(f, "DELAY{time}"),
            Action::WaitForHigh(pin) => {
                write!(f, "WAITFORHIGH{pin}")
            }
            Action::WaitForLow(pin) => {
                write!(f, "WAITFORLOW{pin}")
            }
            Action::SetPullUp(pin) => {
                write!(f, "SETPULLUP{pin}")
            }
            Action::SetPullDown(pin) => {
                write!(f, "SETPULLDOWN{pin}")
            }
        }
    }
}

pub async fn stop_actions(State(appstate): State<AppState>) {
    println!("attempting to stop");
    let _ = log_info(&appstate, "Attempting to Stop");
    appstate.stop_it.store(true, Ordering::Relaxed);
}

pub async fn start_actions(
    State(appstate): State<AppState>,
    Form(input): Form<LoopOption>,
) {
    println!("starting actions...");
    let _ = log_info(&appstate, "Attempting to start actions");
    let should_loop = input.should_loop.as_deref() == Some("true");
    let stop = appstate.stop_it.clone();

    stop.store(false, Ordering::Relaxed);

    loop {
        let gpio = appstate.gpio.lock().unwrap();
        if !gpio.initialized {
            log_error(&appstate, "cannot start loop");
            break;
        }

        let actions = appstate.actions.lock().unwrap().clone();

        if actions.is_empty() {
            println!("actions are empty dawg");
            break;
        }

        for i in actions.iter() {
            println!("{i}");
            let _ = log_info(&appstate, format!("Action: {i}"));
            if stop.load(Ordering::Relaxed) {
                println!("found a stop action");
                let _ = log_info(&appstate, "Found a stop action...");
                break;
            }

            match i {
                Action::SetHigh(pin) => {
                    let mut gpio = appstate.gpio.lock().unwrap();
                    gpio.set_level(*pin, PinLevel::High).unwrap();
                    println!("set high: GPIO {pin}");
                }
                Action::SetLow(pin) => {
                    let mut gpio = appstate.gpio.lock().unwrap();
                    gpio.set_level(*pin, PinLevel::Low).unwrap();
                    println!("set low: GPIO {pin}");
                }
                Action::Delay(time) => {
                    sleep(Duration::from_millis(*time as u64)).await
                }
                Action::WaitForHigh(pin) => loop {
                    let gpio = appstate.gpio.lock().unwrap();
                    if let PinLevel::High = gpio.get_level(*pin).unwrap() {
                        println!("got HIGH signal: GPIO {pin}");
                        break;
                    }
                    drop(gpio);
                },
                Action::WaitForLow(pin) => loop {
                    let gpio = appstate.gpio.lock().unwrap();
                    if let PinLevel::Low = gpio.get_level(*pin).unwrap() {
                        println!("got LOW signal: GPIO {pin}");
                        break;
                    }
                    drop(gpio);
                },
                Action::SetPullUp(pin) => {
                    let mut gpio = appstate.gpio.lock().unwrap();
                    gpio.set_pull_type(*pin, PullType::Up).unwrap();
                    println!("set pullup: GPIO {pin}");
                }
                Action::SetPullDown(pin) => {
                    let mut gpio = appstate.gpio.lock().unwrap();
                    gpio.set_pull_type(*pin, PullType::Down).unwrap();
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

pub async fn delete_action(
    State(appstate): State<AppState>,
    Path(index): Path<usize>,
) {
    let mut actions = appstate.actions.lock().unwrap();
    if index < actions.len() {
        let _ =
            log_info(&appstate, format!("Deleting Action: {}", actions[index]));
        actions.remove(index);

        let clone = actions.clone();
        drop(actions);

        match save_actions(&clone) {
            Err(e) => {
                let _ = log_error(
                    &appstate,
                    format!("Failed to save config: {}", e),
                );
            }
            Ok(_) => println!("rming action, config save"),
        };
    };

    println!("deleting action");
}

pub async fn add_action(
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
            format!("Wait For HIGH GPIO:{}", input.value),
        ),
        "wait-for-low" => (
            Action::WaitForLow(input.value),
            format!("Wait For LOW GPIO:{}", input.value),
        ),
        "set-pull-up" => (
            Action::SetPullUp(input.value),
            format!("GPIO:{} Pull-Up", input.value),
        ),
        "set-pull-down" => (
            Action::SetPullDown(input.value),
            format!("GPIO:{} Pull-Down", input.value),
        ),
        _ => {
            return {
                let _ = log_error(
                    &appstate,
                    format!(
                        "Not a vnotalid action: {}",
                        input.action_type.as_str()
                    ),
                );
                Html("put this in a log somewhere".to_string())
            };
        }
    };

    let mut actions = appstate.actions.lock().unwrap();
    let _ = log_info(&appstate, format!("Adding Action: {}", action));
    actions.push(action.clone());
    let index = actions.len() - 1;

    let clone = actions.clone();
    drop(actions);

    match save_actions(&clone) {
        Err(e) => {
            let _ =
                log_error(&appstate, format!("Failed to save config: {}", e));
        }
        Ok(_) => println!("adding action, config save"),
    };

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

pub async fn get_actions(State(appstate): State<AppState>) -> Html<String> {
    let actions = appstate.actions.lock().unwrap();
    let mut html = String::new();

    for (i, action) in actions.iter().enumerate() {
        let display_text = match action {
            Action::SetHigh(pin) => {
                format!("GPIO:{} Set High", pin)
            }
            Action::SetLow(pin) => {
                format!("GPIO:{} Set Low", pin)
            }
            Action::Delay(time) => {
                format!("Delay {}ms", time)
            }
            Action::WaitForHigh(pin) => {
                format!("Wait For HIGH GPIO:{}", pin)
            }
            Action::WaitForLow(pin) => {
                format!("Wait For LOW GPIO:{}", pin)
            }
            Action::SetPullUp(pin) => {
                format!("GPIO:{} Pull-Up", pin)
            }
            Action::SetPullDown(pin) => {
                format!("GPIO:{} Pull-Down", pin)
            }
        };

        html.push_str(&format!(
            r#"<div class="pin-item" 
            hx-delete="/delete-action/{}" 
            hx-target="closest .pin-item" 
            hx-swap="outerHTML">
                <span class="pin-number">{}</span>
                <span class="pin-delete">DELETE</span>
            </div>"#,
            i, display_text
        ));
    }

    Html(html)
}
