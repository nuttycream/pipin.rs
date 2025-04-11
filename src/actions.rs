use crate::{
    AppState,
    bindings::GpioWrapper,
};

use axum::{
    Form,
    extract::{
        Path,
        State,
    },
    response::Html,
};
use serde::{
    Deserialize,
    Serialize,
};
use std::{
    fmt::{
        self,
        Display,
    },
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

pub async fn stop_actions(State(appstate): State<AppState>) {
    println!("attempting to stop");
    appstate.stop_it.store(true, Ordering::Relaxed);
}

pub async fn start_actions(State(appstate): State<AppState>, Form(input): Form<LoopOption>) {
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

pub async fn delete_action(State(appstate): State<AppState>, Path(index): Path<usize>) {
    let mut actions = appstate.actions.lock().unwrap();
    if index < actions.len() {
        actions.remove(index);
    }
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
