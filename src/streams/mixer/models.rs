use serde_derive::{Deserialize, Serialize};
use serde_json::Value;
use std::{collections::HashMap, convert::TryFrom};

/// An Event coming in from the socket.
///
/// These are sent from Constellation when connecting,
/// receiving a live event, etc.
///
/// See https://dev.mixer.com/reference/constellation/events
#[derive(Debug, Deserialize, Serialize)]
pub struct Event {
    #[serde(rename = "type")]
    pub event_type: String,
    pub event: String,
    /// Data associated with the event. Note that this is,
    /// per the docs, completely unstructured; it depends
    /// on which kind of event was received.
    pub data: Option<Value>,
}

impl TryFrom<Value> for Event {
    type Error = &'static str;

    fn try_from(value: Value) -> Result<Self, Self::Error> {
        let as_text = serde_json::to_string(&value).unwrap();
        let event: Event = match serde_json::from_str(&as_text) {
            Ok(r) => r,
            Err(_) => return Err("Could not load from JSON"),
        };
        Ok(event)
    }
}

impl TryFrom<&str> for Event {
    type Error = &'static str;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        let event: Event = match serde_json::from_str(value) {
            Ok(r) => r,
            Err(_) => return Err("Could not load from JSON"),
        };
        Ok(event)
    }
}

/// A Method to send to the socket.
///
/// This is how clients send data _to_ the socket.
///
/// See https://dev.mixer.com/reference/constellation/methods
#[derive(Debug, Deserialize, Serialize, PartialEq)]
pub struct Method {
    /// Always 'method'
    #[serde(rename = "type")]
    pub method_type: String,
    pub method: String,
    pub params: HashMap<String, Value>,
    /// Unique id for this method call
    pub id: usize,
}

/// Error from Constellation
#[derive(Debug, Deserialize, Serialize, PartialEq)]
pub struct MixerError {
    pub id: u16,
    pub message: String,
}

/// A Replay to a method call.
///
/// These are sent from Constellation to the client as
/// a response to the client sending a method.
///
/// See https://dev.mixer.com/reference/constellation/methods#reply
#[derive(Debug, Deserialize, Serialize)]
pub struct Reply {
    #[serde(rename = "type")]
    /// Which method type this reply is for
    pub reply_type: String,
    /// The id of the method this reply is for
    pub id: usize,
    /// Method call result
    pub result: Option<HashMap<String, Value>>,
    pub error: Option<MixerError>,
}

#[derive(Debug)]
pub enum SocketResponse {
    Event(Event),
    Reply(Reply),
}

impl TryFrom<Value> for Reply {
    type Error = &'static str;

    fn try_from(value: Value) -> Result<Self, Self::Error> {
        let as_text = serde_json::to_string(&value).unwrap();
        let reply: Reply = match serde_json::from_str(&as_text) {
            Ok(r) => r,
            Err(_) => return Err("Could not load from JSON"),
        };
        Ok(reply)
    }
}

impl TryFrom<&str> for Reply {
    type Error = &'static str;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        let reply: Reply = match serde_json::from_str(value) {
            Ok(r) => r,
            Err(_) => return Err("Could not load from JSON"),
        };
        Ok(reply)
    }
}

#[derive(Debug, Default, Deserialize, PartialEq, Eq)]
pub struct Channel {
    pub id: u64,
    #[serde(rename = "userId")]
    pub user_id: u64,
    #[serde(rename = "token")]
    pub name: String,
    pub user: User,
    pub online: bool,
    #[serde(rename = "name")]
    pub title: String,
    #[serde(rename = "bannerUrl")]
    pub banner_url: Option<String>,
    pub thumbnail: Option<Resource>,
    pub cover: Option<Resource>,
    pub badge: Option<Resource>,
    #[serde(rename = "type")]
    pub game: Option<GameType>,
    // #[serde(rename = "viewersTotal")]
    // pub viewers_total: u64,
    // #[serde(rename = "viewersCurrent")]
    // pub viewers_current: u64,
    // #[serde(rename = "numFollowers")]
    // pub num_followers: u64,
    // #[serde(rename = "badgeId")]
    // pub badge_id: Option<u64>,
    // #[serde(rename = "coverId")]
    // pub cover_id: Option<u64>,
    // #[serde(rename = "thumbnailId")]
    // pub thumbnail_id: Option<u64>,
    // #[serde(rename = "typeId")]
    // pub game_type_id: Option<u32>,
    // pub featured: bool,
    // #[serde(rename = "featureLevel")]
    // pub featured_level: i32,
    // pub partnered: bool,
    // pub description: String,
}

#[derive(Debug, Default, Deserialize, PartialEq, Eq)]
pub struct User {
    pub id: u64,
    pub level: u64,
    pub username: String,
    pub experience: u64,
    #[serde(rename = "avatarUrl")]
    pub avatar_url: Option<String>,
    // bio: Option<String>,
}

#[derive(Debug, Default, Deserialize, PartialEq, Eq)]
pub struct GameType {
    pub id: u64,
    pub name: String,
    #[serde(rename = "coverUrl")]
    pub cover_url: Option<String>,
    #[serde(rename = "backgroundUrl")]
    pub background_url: Option<String>,
    // online: u32,
    // #[serde(rename = "viewersCurrent")]
    // viewers_current: u32,
    // parent: String,
    // description: String,
    // source: String,
}

#[derive(Debug, Default, Deserialize, PartialEq, Eq)]
pub struct Resource {
    pub id: u64,
    pub url: String,
    // #[serde(rename = "type")]
    // resource_type: String,
    // relid: u64,
    // store: String,
    // #[serde(rename = "remotePath")]
    // remote_path: String,
}
