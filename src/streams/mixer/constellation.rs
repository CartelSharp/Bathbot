use super::{models::Method, rest::REST, ws_impl::WsStream};
use crate::util::Error;

use async_tungstenite::{
    tokio::connect_async_with_config,
    tungstenite::{handshake::client::Request, protocol::WebSocketConfig, Message},
};
use futures::{stream::SplitSink, SinkExt, StreamExt};
use reqwest::header::HeaderValue;
use serde_json::json;
use std::collections::HashMap;
use tokio::task::JoinHandle;

pub struct Constellation {
    rest: REST,
    read_handle: JoinHandle<()>,
    write: SplitSink<WsStream, Message>,
    curr_id: usize,
}

impl Constellation {
    pub async fn new(client_id: &str) -> Self {
        // Prepare REST client
        let rest = REST::new(client_id);

        // Prepare websocket
        let config = WebSocketConfig {
            max_send_queue: None,
            max_message_size: None,
            max_frame_size: None,
        };
        let mut req = Request::get("wss://constellation.mixer.com");
        let headers = req.headers_mut().unwrap();
        headers.insert("client-id", HeaderValue::from_str(client_id).unwrap());
        headers.insert("x-is-bot", HeaderValue::from_static("true"));
        let (ws_stream, _) = connect_async_with_config(req.body(()).unwrap(), Some(config))
            .await
            .expect("Failed to connect mixer websocket");
        let (write, mut read) = ws_stream.split();
        let read_handle = tokio::spawn(async move {
            // let channels = channels_clone;
            // loop {
            //     match read.recv_json().await {
            //         Ok(Some(SocketResponse::Event(event))) => {
            //             if let Some(data) = event.data {
            //                 if let Some(Value::String(channel)) = data.get("channel") {
            //                     let mut channels = channels.write().await;
            //                     channel
            //                         .split(':')
            //                         .nth(1)
            //                         .and_then(|id| u64::from_str(id).ok())
            //                         .and_then(|id| channels.get_mut(&id))
            //                         .map(|channel| patch_channel(channel, data));
            //                 }
            //             }
            //         }
            //         Ok(Some(SocketResponse::Reply(reply))) => info!("Reply for id {}", reply.id),
            //         Ok(None) => {}
            //         Err(why) => {
            //             debug!("Caught error while receiving: {}", why);
            //             break;
            //         }
            //     }
            // }
        });
        Self {
            rest,
            read_handle,
            write,
            curr_id: 0,
        }
    }

    pub async fn livesubscribe(&mut self, channel: u64) -> Result<(), Error> {
        let mut params = HashMap::new();
        params.insert(
            "events".to_owned(),
            json!(&[&format!("channel:{}:update", channel)]),
        );
        let to_send = Method {
            method_type: "method".to_owned(),
            method: "livesubscribe".to_owned(),
            params,
            id: self.curr_id,
        };
        debug!("Livesubscribing to {} with id {}", channel, self.curr_id);
        let json_str = serde_json::to_string(&to_send)?;
        self.write.send(Message::Text(json_str)).await?;
        self.curr_id += 1;
        Ok(())
    }
}
