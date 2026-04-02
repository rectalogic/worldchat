mod app;
mod chat;

use std::{
    cell::{OnceCell, RefCell},
    str::FromStr,
    sync::{Arc, Mutex},
};

pub use app::AppPlugin;
use bevy::{prelude::*, tasks::IoTaskPool};
use futures_util::{
    Sink, SinkExt, Stream, StreamExt,
    stream::{SplitSink, SplitStream},
};
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub fn start() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .run();

    // App::new().add_plugins(AppPlugin).run();
}

fn setup() {
    IoTaskPool::get()
        .spawn(async move {
            let ws = tokio_tungstenite_wasm::connect_with_protocols(
                "wss://fiery.swiftirc.net:4443",
                &["text.ircv3.net"],
            )
            .await
            .unwrap();
            let (mut tx, mut rx) = ws.split();

            // Send a CAP END to signify that we're IRCv3-compliant (and to end negotiations!).
            let cap = irc_proto::command::Command::CAP(
                None,
                irc_proto::command::CapSubCommand::END,
                None,
                None,
            );
            let s = String::from(&cap);
            tx.send(tokio_tungstenite_wasm::Message::text(&s))
                .await
                .unwrap();

            let user =
                irc_proto::command::Command::USER("user007".into(), "0".into(), "user007".into());
            let s = String::from(&user);
            tx.send(tokio_tungstenite_wasm::Message::text(&s))
                .await
                .unwrap();

            let nick = irc_proto::command::Command::NICK("user007".into());
            let s = String::from(&nick);
            tx.send(tokio_tungstenite_wasm::Message::text(&s))
                .await
                .unwrap();

            while let Some(response) = rx.next().await {
                if let Ok(tokio_tungstenite_wasm::Message::Text(bytes)) = response
                    && let Ok(message) =
                        irc_proto::message::Message::from_str(bytes.to_string().as_str())
                {
                    info!("{message:?}");
                    match message.command {
                        irc_proto::command::Command::PING(server1, server2) => {
                            let pong = irc_proto::command::Command::PONG(server1, server2);
                            let s = String::from(&pong);
                            tx.send(tokio_tungstenite_wasm::Message::text(&s))
                                .await
                                .unwrap();
                        }
                        irc_proto::command::Command::UserMODE(..) => {
                            let join = irc_proto::command::Command::JOIN(
                                "#bevyworldchat".into(),
                                None,
                                None,
                            );
                            let s = String::from(&join);
                            tx.send(tokio_tungstenite_wasm::Message::text(&s))
                                .await
                                .unwrap();
                            info!("joined channel");
                        }
                        _ => {}
                    }
                }
            }
        })
        .detach();
}
