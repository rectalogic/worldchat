use std::time::SystemTime;

use bevy::prelude::*;
use iroh::{EndpointId, PublicKey, SecretKey, Signature};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub enum ChatMessage {
    Message(String),
}

#[derive(Debug)]
pub enum GossipEvent {
    NeighborUp(EndpointId),
    NeighborDown(EndpointId),
    Message(ReceivedMessage),
    Lagged,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SignedMessage {
    from: PublicKey,
    data: Vec<u8>,
    signature: Signature,
}

impl SignedMessage {
    pub fn verify_and_decode(bytes: &[u8]) -> Result<ReceivedMessage, BevyError> {
        let signed_message: Self = postcard::from_bytes(bytes)?;
        let key: PublicKey = signed_message.from;
        key.verify(&signed_message.data, &signed_message.signature)?;
        let message: WireMessage = postcard::from_bytes(&signed_message.data)?;
        let WireMessage::VO { timestamp, message } = message;
        Ok(ReceivedMessage {
            from: signed_message.from,
            timestamp,
            message,
        })
    }

    pub fn sign_and_encode(
        secret_key: &SecretKey,
        message: ChatMessage,
    ) -> Result<Vec<u8>, BevyError> {
        let timestamp = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_micros() as u64;
        let wire_message = WireMessage::VO { timestamp, message };
        let data = postcard::to_stdvec(&wire_message)?;
        let signature = secret_key.sign(&data);
        let from: PublicKey = secret_key.public();
        let signed_message = Self {
            from,
            data,
            signature,
        };
        let encoded = postcard::to_stdvec(&signed_message)?;
        Ok(encoded)
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub enum WireMessage {
    VO {
        timestamp: u64,
        message: ChatMessage,
    },
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ReceivedMessage {
    timestamp: u64,
    from: EndpointId,
    message: ChatMessage,
}

impl TryFrom<iroh_gossip::api::Event> for GossipEvent {
    type Error = BevyError;
    fn try_from(event: iroh_gossip::api::Event) -> Result<Self, Self::Error> {
        match event {
            iroh_gossip::api::Event::NeighborUp(endpoint_id) => {
                Ok(GossipEvent::NeighborUp(endpoint_id))
            }
            iroh_gossip::api::Event::NeighborDown(endpoint_id) => {
                Ok(GossipEvent::NeighborDown(endpoint_id))
            }
            iroh_gossip::api::Event::Received(message) => Ok(GossipEvent::Message(
                SignedMessage::verify_and_decode(&message.content)?,
            )),
            iroh_gossip::api::Event::Lagged => Ok(GossipEvent::Lagged),
        }
    }
}
