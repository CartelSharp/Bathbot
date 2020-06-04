use crate::{
    database::{MySQL, Platform},
    embeds::BasicEmbedData,
    streams::{Mixer, Twitch, TwitchStream},
    structs::{Guilds, OnlineTwitch, ReactionTracker, StreamTracks},
    util::discord::get_member,
    WITH_STREAM_TRACK,
};

use rayon::prelude::*;
use serenity::{
    async_trait,
    http::Http,
    model::{
        channel::Reaction,
        event::ResumedEvent,
        gateway::{Activity, Ready},
        guild::Guild,
        id::{ChannelId, GuildId, RoleId},
        voice::VoiceState,
    },
    prelude::*,
};
use std::{
    collections::{HashMap, HashSet},
    env,
    sync::{Arc, Once},
};
use strfmt::strfmt;
use tokio::time;

static START: Once = Once::new();

pub struct Handler;

#[async_trait]
impl EventHandler for Handler {
    async fn ready(&self, ctx: Context, ready: Ready) {
        // Tracking streams
        let mut track_streams = false;
        START.call_once(|| {
            track_streams = true;
        });
        if track_streams && WITH_STREAM_TRACK {
            let mixer_tracks: Vec<_> = {
                let data = ctx.data.read().await;
                data.get::<MySQL>()
                    .expect("Could not get MySQL")
                    .get_stream_tracks()
                    .unwrap_or_else(|why| panic!("Could not get stream_tracks: {}", why))
                    .iter()
                    .filter(|track| track.platform == Platform::Mixer)
                    .map(|track| track.user_id)
                    .collect()
            };
            let mixer_client_id =
                env::var("MIXER_CLIENT_ID").expect("Could not load MIXER_CLIENT_ID");
            let mixer = Mixer::new(&mixer_client_id, mixer_tracks, &ctx)
                .await
                .expect("Could not initialize Mixer");
            {
                let mut data = ctx.data.write().await;
                data.insert::<Mixer>(mixer);
            }
            let http = Arc::clone(&ctx.http);
            let data = Arc::clone(&ctx.data);
            let _ = tokio::spawn(async move {
                let track_delay = 10;
                let mut interval = time::interval(time::Duration::from_secs(track_delay * 60));
                interval.tick().await;
                loop {
                    _check_streams(&http, Arc::clone(&data).clone()).await;
                    interval.tick().await;
                }
            });
            info!("Stream tracking started");
        } else {
            info!("Stream tracking skipped");
        }

        if let Some(shard) = ready.shard {
            info!(
                "{} is connected on shard {}/{}",
                ready.user.name, shard[0], shard[1],
            );
        } else {
            info!("Connected as {}", ready.user.name);
        }
        ctx.set_activity(Activity::playing("osu! (<help)")).await;
    }

    async fn resume(&self, _: Context, _: ResumedEvent) {
        info!("Resumed connection");
    }

    async fn guild_create(&self, ctx: Context, guild: Guild, is_new: bool) {
        if is_new {
            // Insert basic guild info into database
            let guild = {
                let data = ctx.data.read().await;
                let mysql = data.get::<MySQL>().expect("Could not get MySQL");
                match mysql.insert_guild(guild.id.0) {
                    Ok(g) => {
                        debug!(
                            "Inserted new guild {} with id {} to DB",
                            guild.name, guild.id
                        );
                        Some(g)
                    }
                    Err(why) => {
                        error!(
                            "Could not insert new guild {} with id {} to DB: {}",
                            guild.name, guild.id, why
                        );
                        None
                    }
                }
            };
            if let Some(guild) = guild {
                let mut data = ctx.data.write().await;
                let guilds = data.get_mut::<Guilds>().expect("Could not get Guilds");
                guilds.insert(guild.guild_id, guild);
            }
        }
    }

    async fn voice_state_update(
        &self,
        ctx: Context,
        guild: Option<GuildId>,
        old: Option<VoiceState>,
        new: VoiceState,
    ) {
        // Try assigning the server's VC role to the member
        if let Some(guild_id) = guild {
            let role = {
                let data = ctx.data.read().await;
                data.get::<Guilds>()
                    .and_then(|guilds| guilds.get(&guild_id))
                    .and_then(|guild| guild.vc_role)
            };
            // If the server has configured such a role
            if let Some(role) = role {
                // Get the event's member
                let mut member = match guild_id.member(&ctx, new.user_id).await {
                    Ok(member) => member,
                    Err(why) => {
                        warn!("Could not get member for VC update: {}", why);
                        return;
                    }
                };
                let role_name = role
                    .to_role_cached(&ctx.cache)
                    .await
                    .expect("Role not found in cache")
                    .name;
                // If either the member left VC, or joined the afk channel
                let remove_role = match new.channel_id {
                    None => true,
                    Some(channel) => channel
                        .name(&ctx.cache)
                        .await
                        .map_or(false, |name| &name.to_lowercase() == "afk"),
                };
                if remove_role {
                    // Remove role
                    if let Err(why) = member.remove_role(&ctx.http, role).await {
                        error!("Could not remove role from member for VC update: {}", why);
                    } else {
                        info!(
                            "Removed role '{}' from member {}",
                            role_name, member.user.name
                        );
                    }
                } else {
                    // Add role if the member is either coming from the afk channel
                    // or hasn't been in a VC before
                    let add_role = match old {
                        None => true,
                        Some(old_state) => old_state
                            .channel_id
                            .unwrap()
                            .name(&ctx.cache)
                            .await
                            .map_or(true, |name| &name.to_lowercase() == "afk"),
                    };
                    if add_role {
                        if let Err(why) = member.add_role(&ctx.http, role).await {
                            error!("Could not add role to member for VC update: {}", why);
                        } else {
                            info!(
                                "Assigned role '{}' to member {}",
                                role_name, member.user.name
                            );
                        }
                    }
                }
            }
        }
    }

    async fn reaction_add(&self, ctx: Context, reaction: Reaction) {
        // Check if the reacting user now gets a role
        let key = (reaction.channel_id, reaction.message_id);
        let role: Option<RoleId> = match ctx.data.read().await.get::<ReactionTracker>() {
            Some(tracker) => {
                if tracker.contains_key(&key) {
                    Some(*tracker.get(&key).unwrap())
                } else {
                    None
                }
            }
            None => {
                error!("Could not get ReactionTracker");
                return;
            }
        };
        if let Some(role) = role {
            if let Some(mut member) = get_member(&ctx, reaction.channel_id, reaction.user_id).await
            {
                let role_name = role
                    .to_role_cached(&ctx.cache)
                    .await
                    .expect("Role not found in cache")
                    .name;
                if let Err(why) = member.add_role(&ctx.http, role).await {
                    error!("Could not add role to member for reaction: {}", why);
                } else {
                    info!(
                        "Assigned role '{}' to member {}",
                        role_name, member.user.name
                    );
                }
            }
        }
    }

    async fn reaction_remove(&self, ctx: Context, reaction: Reaction) {
        // Check if the reacting user now loses a role
        let key = (reaction.channel_id, reaction.message_id);
        let role = match ctx.data.read().await.get::<ReactionTracker>() {
            Some(tracker) => {
                if tracker.contains_key(&key) {
                    Some(*tracker.get(&key).unwrap())
                } else {
                    None
                }
            }
            None => {
                error!("Could not get ReactionTracker");
                return;
            }
        };
        if let Some(role) = role {
            if let Some(mut member) = get_member(&ctx, reaction.channel_id, reaction.user_id).await
            {
                let role_name = role
                    .to_role_cached(&ctx.cache)
                    .await
                    .expect("Role not found in cache")
                    .name;
                if let Err(why) = member.remove_role(&ctx.http, role).await {
                    error!("Could not remove role from member for reaction: {}", why);
                } else {
                    info!(
                        "Removed role '{}' from member {}",
                        role_name, member.user.name
                    );
                }
            }
        }
    }
}

async fn _check_streams(http: &Http, data: Arc<RwLock<TypeMap>>) {
    let now_online = {
        let reading = data.read().await;

        // Get data about what needs to be tracked for which channel
        let stream_tracks = reading
            .get::<StreamTracks>()
            .expect("Could not get StreamTracks");
        let user_ids: Vec<_> = stream_tracks
            .iter()
            .filter(|track| track.platform == Platform::Twitch)
            .map(|track| track.user_id)
            .collect();
        // Twitch provides up to 100 streams per request, otherwise its trimmed
        if user_ids.len() > 100 {
            warn!("Reached 100 twitch trackings, improve handling!");
        }

        // Get stream data about all streams that need to be tracked
        let twitch = reading.get::<Twitch>().expect("Could not get Twitch");
        let mut streams = match twitch.get_streams(&user_ids).await {
            Ok(streams) => streams,
            Err(why) => {
                warn!("Error while retrieving streams: {}", why);
                return;
            }
        };

        // Filter streams whether they're live
        streams.retain(TwitchStream::is_live);
        let online_streams = reading
            .get::<OnlineTwitch>()
            .expect("Could not get OnlineTwitch");
        let now_online: HashSet<_> = streams.iter().map(|stream| stream.user_id).collect();

        // If there was no activity change since last time, don't do anything
        if &now_online == online_streams {
            None
        } else {
            // Filter streams whether its already known they're live
            streams.retain(|stream| !online_streams.contains(&stream.user_id));

            let ids: Vec<_> = streams.iter().map(|s| s.user_id).collect();
            let users: HashMap<_, _> = match twitch.get_users(&ids).await {
                Ok(users) => users.into_iter().map(|u| (u.user_id, u)).collect(),
                Err(why) => {
                    warn!("Error while retrieving twitch users: {}", why);
                    return;
                }
            };

            let mut fmt_data = HashMap::new();
            fmt_data.insert(String::from("width"), String::from("360"));
            fmt_data.insert(String::from("height"), String::from("180"));

            // Put streams into a more suitable data type and process the thumbnail url
            let streams: HashMap<u64, TwitchStream> = streams
                .into_par_iter()
                .map(|mut stream| {
                    if let Ok(thumbnail) = strfmt(&stream.thumbnail_url, &fmt_data) {
                        stream.thumbnail_url = thumbnail;
                    }
                    (stream.user_id, stream)
                })
                .collect();

            // Process each tracking by notifying corresponding channels
            for track in stream_tracks {
                if streams.contains_key(&track.user_id) {
                    let stream = streams.get(&track.user_id).unwrap();
                    let data = BasicEmbedData::create_stream_twitch_notif(
                        stream,
                        users.get(&stream.user_id).unwrap(),
                    );
                    let _ = ChannelId(track.channel_id)
                        .send_message(http, |m| m.embed(|e| data.build(e)))
                        .await;
                }
            }
            Some(now_online)
        }
    };
    if let Some(now_online) = now_online {
        let mut writing = data.write().await;
        let online_twitch = writing
            .get_mut::<OnlineTwitch>()
            .expect("Could not get OnlineTwitch");
        online_twitch.clear();
        for id in now_online {
            online_twitch.insert(id);
        }
    }
}
