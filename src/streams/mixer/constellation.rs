use super::{
    models::{Channel, Method, SocketResponse},
    rest::REST,
    ws_impl::*,
};
use crate::{embeds::BasicEmbedData, util::Error, MixerChannels, Platform, StreamTracks};

use async_tungstenite::{
    tokio::connect_async,
    tungstenite::{handshake::client::Request, Message},
};
use futures::{stream::SplitSink, SinkExt, StreamExt};
use reqwest::header::HeaderValue;
use serde_json::{json, Value};
use serenity::{
    http::Http,
    model::id::ChannelId,
    prelude::{Context, TypeMap},
};
use std::{collections::HashMap, str::FromStr, sync::Arc};
use tokio::{sync::RwLock, task::JoinHandle, time};

type ChannelMap = Arc<RwLock<HashMap<u64, Channel>>>;

pub struct Mixer {
    rest: REST,
    write: Arc<RwLock<SplitSink<WsStream, Message>>>,
    curr_id: usize,
    channels: ChannelMap,
    _read_handle: JoinHandle<()>,
    _ping_handle: JoinHandle<()>,
}

impl Mixer {
    pub async fn new(client_id: &str, tracks: Vec<u64>, ctx: &Context) -> Result<Self, Error> {
        // Prepare REST client
        let rest = REST::new(client_id);

        // Prepare channels
        let channels = get_channels(&rest, tracks)
            .await?
            .into_iter()
            .map(|channel| (channel.id, channel))
            .collect();
        let channels = Arc::new(RwLock::new(channels));
        {
            let mut data = ctx.data.write().await;
            data.insert::<MixerChannels>(Arc::clone(&channels));
        }

        // Prepare websocket
        let mut req = Request::get("wss://constellation.mixer.com");
        let headers = req.headers_mut().unwrap();
        headers.insert("client-id", HeaderValue::from_str(client_id).unwrap());
        headers.insert("x-is-bot", HeaderValue::from_static("true"));
        let (ws_stream, _) = connect_async(req.body(()).unwrap())
            .await
            .expect("Failed to connect mixer websocket");
        let (write, mut read) = ws_stream.split();

        // Start up the async worker that reads websocket messages
        let channels_clone = Arc::clone(&channels);
        let http = Arc::clone(&ctx.http);
        let data = Arc::clone(&ctx.data);
        let _read_handle = tokio::spawn(async move {
            let channels: ChannelMap = channels_clone;
            loop {
                match read.recv_json().await {
                    Ok(Some(SocketResponse::Event(event))) => {
                        if let Some(event_data) = event.data {
                            if let Some(Value::String(channel)) = event_data.get("channel") {
                                let id = channel
                                    .split(':')
                                    .nth(1)
                                    .and_then(|id| u64::from_str(id).ok());
                                let mut channels = channels.write().await;
                                if let Some(ref mut channel) =
                                    id.and_then(|id| channels.get_mut(&id))
                                {
                                    if patch_channel(channel, event_data) {
                                        send_notifs(&http, &data, channel).await;
                                    }
                                }
                            }
                        }
                    }
                    Ok(Some(SocketResponse::Reply(reply))) => match reply.error {
                        None => debug!("Successful reply for id {}", reply.id),
                        Some(error) => warn!("Error reply for id {}: {}", reply.id, error.message),
                    },
                    Ok(None) => {}
                    Err(why) => {
                        warn!("Caught error while receiving: {}", why);
                        break;
                    }
                }
            }
        });

        // Start up the async worker that pings regularly
        let write = Arc::new(RwLock::new(write));
        let write_clone = Arc::clone(&write);
        let _ping_handle = tokio::spawn(async move {
            let write = write_clone;
            let mut interval = time::interval(time::Duration::from_secs(18));
            interval.tick().await;
            loop {
                interval.tick().await;
                let mut write = write.write().await;
                if let Err(why) = write.send(Message::Ping(vec![])).await {
                    warn!("Error while sending ping: {}", why);
                }
            }
        });
        let mut result = Self {
            rest,
            write,
            curr_id: 0,
            channels,
            _read_handle,
            _ping_handle,
        };

        // Subscribe to all stored channels
        let mut interval = time::interval(time::Duration::from_millis(250));
        let channels: Vec<_> = {
            let channels = result.channels.read().await;
            channels.keys().copied().collect()
        };
        for id in channels {
            interval.tick().await;
            result.subscribe(id).await?;
        }
        Ok(result)
    }

    pub async fn channel(&self, channel: &str) -> Result<Channel, Error> {
        self.rest.channel_by_name(channel).await
    }

    pub async fn track_stream(&mut self, channel_id: u64) -> Result<(), Error> {
        self.subscribe(channel_id).await?;
        let channel = self.rest.channel_by_id(channel_id).await?;
        {
            let mut channels = self.channels.write().await;
            channels.entry(channel.id).or_insert(channel);
        }
        Ok(())
    }

    async fn subscribe(&mut self, channel: u64) -> Result<(), Error> {
        self.update_event(channel, "livesubscribe").await?;
        debug!("Subscribing to {} with id {}", channel, self.curr_id - 1);
        Ok(())
    }

    pub async fn unsubscribe(&mut self, channel: u64) -> Result<(), Error> {
        self.update_event(channel, "liveunsubscribe").await?;
        debug!(
            "Unsubscribing from {} with id {}",
            channel,
            self.curr_id - 1
        );
        Ok(())
    }

    async fn update_event(&mut self, channel: u64, event_type: &str) -> Result<(), Error> {
        let mut params = HashMap::new();
        params.insert(
            "events".to_owned(),
            json!(&[&format!("channel:{}:update", channel)]),
        );
        self.call_method(event_type, params).await?;
        Ok(())
    }

    async fn call_method(
        &mut self,
        method: &str,
        params: HashMap<String, Value>,
    ) -> Result<(), Error> {
        let to_send = Method {
            method_type: "method".to_owned(),
            method: method.to_owned(),
            params,
            id: self.curr_id,
        };
        let mut write = self.write.write().await;
        write.send_json(&to_send).await?;
        self.curr_id = self.curr_id.wrapping_add(1);
        Ok(())
    }
}

macro_rules! patch_field {
    ($channel: expr, $field: tt, $val: expr) => {
        match serde_json::from_value($val) {
            Ok(parsed) => $channel.$field = parsed,
            Err(why) => warn!("Error while parsing: {}", why),
        }
    };
}

/// Parse `data` and modify `channel`'s appropriate field
///
/// Returns `true` if the `online` field was changed
fn patch_channel(channel: &mut Channel, mut data: Value) -> bool {
    if let Value::Object(payload) = data["payload"].take() {
        if let Some((key, value)) = payload.into_iter().next() {
            match key.as_str() {
                "online" => {
                    patch_field!(channel, online, value);
                    return channel.online;
                }
                "user" => patch_field!(channel, user, value),
                "name" => patch_field!(channel, title, value),
                "bannerUrl" => patch_field!(channel, banner_url, value),
                "thumbnail" => patch_field!(channel, thumbnail, value),
                "cover" => patch_field!(channel, cover, value),
                "badge" => patch_field!(channel, badge, value),
                "type" => patch_field!(channel, game, value),
                _ => {} // Irrelevant event
            }
        }
    }
    false
}

async fn get_channels(rest: &REST, mut ids: Vec<u64>) -> Result<Vec<Channel>, Error> {
    let mut channels = rest.channels(&ids).await?;
    ids.retain(|id| channels.iter().all(|channel| channel.id != *id));
    let mut interval = time::interval(time::Duration::from_millis(250));
    for id in ids {
        interval.tick().await; // rate limiting
        channels.push(rest.channel_by_id(id).await?);
    }
    Ok(channels)
}

async fn send_notifs(http: &Arc<Http>, data: &Arc<RwLock<TypeMap>>, channel: &Channel) {
    let data = data.read().await;
    let stream_tracks = data
        .get::<StreamTracks>()
        .expect("Could not get StreamTracks")
        .iter()
        .filter(|track| track.user_id == channel.id && track.platform == Platform::Mixer);
    for track in stream_tracks {
        let embed = BasicEmbedData::create_stream_mixer_notif(channel);
        if let Err(why) = ChannelId(track.channel_id)
            .send_message(http, |m| m.embed(|e| embed.build(e)))
            .await
        {
            warn!(
                "Error while sending mixer online notification for {}: {}",
                channel.name, why
            );
        }
    }
}
