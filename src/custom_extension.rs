use std::sync::mpsc::Sender;

use deno_core::error::AnyError;
use deno_core::op_sync;
use deno_core::serde::Deserialize;
use deno_core::serde::Serialize;
use deno_core::Extension;
use deno_core::OpState;

use crate::AstrodonMessage;

pub fn new(sender: Sender<AstrodonMessage>) -> Extension {
    Extension::builder()
        .ops(vec![
            ("runWindow", op_sync(run_window)),
            ("sendToWindow", op_sync(send_to_window)),
        ])
        .state(move |s| {
            s.put(sender.clone());
            Ok(())
        })
        .build()
}

#[derive(Serialize, Deserialize, Debug)]
pub struct RunWindowMessage {
    pub id: String,
    pub title: String,
    pub url: String,
}

fn run_window(state: &mut OpState, args: RunWindowMessage, _: ()) -> Result<(), AnyError> {
    let s: &Sender<AstrodonMessage> = state.try_borrow::<Sender<AstrodonMessage>>().unwrap();
    s.send(AstrodonMessage::RunWindowMessage(args)).unwrap();
    Ok(())
}

#[derive(Serialize, Deserialize, Debug)]
pub struct SentToWindowMessage {
    pub id: String,
    pub event: String,
    pub content: String,
}

fn send_to_window(state: &mut OpState, args: SentToWindowMessage, _: ()) -> Result<(), AnyError> {
    let s: &Sender<AstrodonMessage> = state.try_borrow::<Sender<AstrodonMessage>>().unwrap();
    s.send(AstrodonMessage::SentToWindowMessage(args)).unwrap();
    Ok(())
}
