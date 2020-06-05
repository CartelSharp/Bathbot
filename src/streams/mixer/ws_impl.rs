use super::models::{Event, Reply, SocketResponse};
use crate::util::Error;

use async_tungstenite::{
    stream::Stream, tokio::TokioAdapter, tungstenite::Message, WebSocketStream,
};
use futures::{
    stream::{SplitSink, SplitStream},
    SinkExt, StreamExt, TryStreamExt,
};
use serde::Serialize;
use serenity::async_trait;
use std::convert::TryFrom;
use tokio::{
    net::TcpStream,
    time::{timeout, Duration},
};
use tokio_tls::TlsStream;

pub type WsStream = WebSocketStream<
    Stream<TokioAdapter<TcpStream>, TokioAdapter<TlsStream<TokioAdapter<TokioAdapter<TcpStream>>>>>,
>;
type Result<T> = std::result::Result<T, Error>;

const TIMEOUT: Duration = Duration::from_secs(10);

#[async_trait]
pub trait ReceiverExt {
    async fn recv_json(&mut self) -> Result<Option<SocketResponse>>;
    async fn try_recv_json(&mut self) -> Result<Option<SocketResponse>>;
}

#[async_trait]
pub trait SenderExt {
    async fn send_json<T: Send + Sync + Serialize>(&mut self, value: &T) -> Result<()>;
}

#[async_trait]
impl ReceiverExt for SplitStream<WsStream> {
    async fn recv_json(&mut self) -> Result<Option<SocketResponse>> {
        let ws_message = match timeout(TIMEOUT, self.next()).await {
            Ok(Some(v)) => v.ok(),
            Ok(None) => return Err(Error::Custom("WsStream empty".to_string())),
            Err(_) => None,
        };
        convert_ws_message(ws_message)
    }

    async fn try_recv_json(&mut self) -> Result<Option<SocketResponse>> {
        convert_ws_message(self.try_next().await.ok().flatten())
    }
}

#[async_trait]
impl SenderExt for SplitSink<WsStream, Message> {
    async fn send_json<T: Send + Sync + Serialize>(&mut self, value: &T) -> Result<()> {
        Ok(serde_json::to_string(value)
            .map(Message::Text)
            .map_err(Error::from)
            .and_then(|m| Ok(self.send(m)))?
            .await?)
    }
}

#[inline]
fn convert_ws_message(message: Option<Message>) -> Result<Option<SocketResponse>> {
    Ok(match message {
        Some(Message::Text(payload)) => {
            let parsed = Event::try_from(payload.as_str())
                .map(SocketResponse::Event)
                .or_else(|_| Reply::try_from(payload.as_str()).map(SocketResponse::Reply));
            match parsed {
                Ok(resp) => Some(resp),
                Err(why) => {
                    warn!("Error deserializing text: {:?}; text: {}", why, payload);
                    None
                }
            }
        }
        Some(Message::Close(Some(frame))) => {
            return Err(Error::Custom(format!(
                "Closing through Message::Close({})",
                frame
            )))
        }
        _ => None,
    })
}
