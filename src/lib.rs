mod app;
mod chat;
mod irc;

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
use irc_proto::{
    command::{CapSubCommand, Command},
    response::Response,
};
use tokio_tungstenite_wasm as ws;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub fn start() {
    App::new()
        .add_plugins((
            DefaultPlugins,
            irc::IrcPlugin {
                server: "wss://fiery.swiftirc.net:4443".into(),
                user: "u007".into(),
            },
        ))
        .add_systems(Startup, setup)
        .run();

    // App::new().add_plugins(AppPlugin).run();
}

struct WsSender(SplitSink<ws::WebSocketStream, ws::Message>);

impl WsSender {
    async fn send(&mut self, command: &Command) -> ws::error::Result<()> {
        self.0.send(ws::Message::text(String::from(command))).await
    }
}

fn setup() {
    IoTaskPool::get()
        .spawn(async move {
            let ws =
                ws::connect_with_protocols("wss://fiery.swiftirc.net:4443", &["text.ircv3.net"])
                    .await
                    .unwrap();
            let (tx, mut rx) = ws.split();
            let mut tx = WsSender(tx);

            // Send a CAP END to signify that we're IRCv3-compliant (and to end negotiations!).
            tx.send(&Command::CAP(None, CapSubCommand::END, None, None))
                .await
                .unwrap();

            tx.send(&Command::USER(
                "user007".into(),
                "0".into(),
                "user007".into(),
            ))
            .await
            .unwrap();

            tx.send(&Command::NICK("user007".into())).await.unwrap();

            while let Some(response) = rx.next().await {
                if let Ok(ws::Message::Text(bytes)) = response
                    && let Ok(message) =
                        irc_proto::message::Message::from_str(bytes.to_string().as_str())
                {
                    info!("{message:?}");
                    match message.command {
                        Command::PING(server1, server2) => {
                            tx.send(&Command::PONG(server1, server2)).await.unwrap();
                        }
                        Command::Response(Response::RPL_ENDOFMOTD, _)
                        | Command::Response(Response::ERR_NOMOTD, _) => {
                            tx.send(&Command::JOIN("#bevyworldchat".into(), None, None))
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
