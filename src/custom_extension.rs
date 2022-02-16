use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::Arc;

use deno_core::anyhow::Error;
use deno_core::error::AnyError;
use deno_core::op_async;
use deno_core::op_sync;
use deno_core::serde::Deserialize;
use deno_core::serde::Serialize;
use deno_core::Extension;
use deno_core::OpState;
use tokio::sync::mpsc;
use tokio::sync::mpsc::UnboundedSender;
use tokio::sync::Mutex;
use uuid::Uuid;

use crate::AstrodonMessage;

#[derive(Serialize, Deserialize, Debug)]
struct EventListen {
    name: String,
}

async fn listen_event(
    state: Rc<RefCell<OpState>>,
    sett: EventListen,
    _: (),
) -> Result<String, Error> {
    let (s, mut r) = mpsc::unbounded_channel();
    let s_id = Uuid::new_v4();

    let state = state.borrow();

    let subs: &Arc<Mutex<HashMap<String, HashMap<Uuid, UnboundedSender<String>>>>> = state
        .try_borrow::<Arc<Mutex<HashMap<String, HashMap<Uuid, UnboundedSender<String>>>>>>()
        .unwrap();

    subs.lock()
        .await
        .try_insert(sett.name.clone(), HashMap::new())
        .ok();
    subs.lock()
        .await
        .get_mut(&sett.name)
        .unwrap()
        .insert(s_id.clone(), s.clone());

    let event = r.recv().await;

    subs.lock().await.get_mut(&sett.name).unwrap().remove(&s_id);

    // TODO, remove the event hashmap if no more senders are on it

    Ok(event.unwrap())
}

pub fn new(
    sender: UnboundedSender<AstrodonMessage>,
    subs: Arc<Mutex<HashMap<String, HashMap<Uuid, UnboundedSender<String>>>>>,
) -> Extension {
    Extension::builder()
        .ops(vec![
            ("runWindow", op_sync(run_window)),
            ("sendToWindow", op_sync(send_to_window)),
            ("listenEvent", op_async(listen_event)),
        ])
        .state(move |s| {
            s.put(sender.clone());
            s.put(subs.clone());
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
    let s: &UnboundedSender<AstrodonMessage> = state
        .try_borrow::<UnboundedSender<AstrodonMessage>>()
        .unwrap();
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
    let s: &UnboundedSender<AstrodonMessage> = state
        .try_borrow::<UnboundedSender<AstrodonMessage>>()
        .unwrap();
    s.send(AstrodonMessage::SentToWindowMessage(args)).unwrap();
    Ok(())
}
