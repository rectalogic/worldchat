mod app;
mod chat;

use std::{
    cell::{OnceCell, RefCell},
    sync::{Arc, Mutex},
};

pub use app::AppPlugin;
use bevy::prelude::*;
use wasm_bindgen::prelude::*;

thread_local! {
    pub static WEBSOCKET: OnceCell<RefCell<(ewebsock::WsSender, ewebsock::WsReceiver)>> = const { OnceCell::new() };
}

#[wasm_bindgen]
pub fn start() {
    let (tx, rx) = ewebsock::connect(
        "wss://fiery.swiftirc.net:4443",
        ewebsock::Options {
            subprotocols: vec!["text.ircv3.net".into()],
            ..default()
        },
    )
    .unwrap();

    WEBSOCKET.with(|cell| cell.set(RefCell::new((tx, rx))));

    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Update, move || {
            WEBSOCKET.with(|cell| {
                if let Some(ws) = cell.get()
                    && let (tx, rx) = &mut *ws.borrow_mut()
                    && let Some(ewebsock::WsEvent::Opened) = rx.try_recv()
                {
                    let join =
                        irc_proto::command::Command::JOIN("#bevyworldchat".into(), None, None);
                    tx.send(ewebsock::WsMessage::Text((&join).into()));
                }
            });
        })
        .run();
    // App::new().add_plugins(AppPlugin).run();
}
