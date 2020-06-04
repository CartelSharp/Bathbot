use crate::{
    commands::checks::*,
    database::{Platform, StreamTrack},
    util::discord,
    Mixer, MixerChannels, MySQL, StreamTracks, TwitchUsers,
};

use serenity::{
    framework::standard::{macros::command, Args, CommandResult},
    model::prelude::Message,
    prelude::{Context, RwLock, TypeMap},
};
use std::sync::Arc;

#[command]
#[checks(Authority)]
#[description = "Let me no longer notify this channel when the given stream comes online"]
#[aliases("streamremove")]
#[usage = "[twitch / mixer] [stream name]"]
#[example = "twitch loltyler1"]
async fn removestream(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    // Parse the platform and stream name
    if args.len() < 2 {
        msg.channel_id
            .say(
                ctx,
                "The first argument must be either `twitch` or `mixer`. \
                The next argument must be the name of the stream.",
            )
            .await?;
        return Ok(());
    }
    let platform = match args.single::<String>()?.to_lowercase().as_str() {
        "twitch" => Platform::Twitch,
        "mixer" => Platform::Mixer,
        _ => {
            msg.channel_id
                .say(
                    ctx,
                    "The first argument must be either `twitch` or `mixer`. \
                     The next argument must be the name of the stream.",
                )
                .await?;
            return Ok(());
        }
    };
    let name = args.single::<String>()?.to_lowercase();
    let removed = match platform {
        Platform::Mixer => handle_mixer_remove(&name, msg.channel_id.0, &ctx.data).await,
        Platform::Twitch => handle_twitch_remove(&name, msg.channel_id.0, &ctx.data).await,
    };

    let content = match removed {
        Ok(removed) => {
            if removed {
                format!(
                    "I'm no longer tracking `{}`'s {:?} stream in this channel",
                    name, platform
                )
            } else {
                "That stream wasn't tracked anyway".to_string()
            }
        }
        Err(err_msg) => err_msg,
    };
    let response = msg.channel_id.say(ctx, content).await?;
    discord::reaction_deletion(&ctx, response, msg.author.id).await;
    Ok(())
}

async fn handle_mixer_remove(
    name: &str,
    channel_id: u64,
    data: &Arc<RwLock<TypeMap>>,
) -> Result<bool, String> {
    // Is channel tracked anywhere?
    let mixer_id = {
        let data = data.read().await;
        let channels_lock = data
            .get::<MixerChannels>()
            .expect("Could not get MixerChannels");
        let channels = channels_lock.read().await;
        channels
            .iter()
            .find(|(_, channel)| channel.name.to_lowercase() == name)
            .map(|(id, _)| *id)
    };
    if let Some(mixer_id) = mixer_id {
        let mut data = data.write().await;
        let stream_tracks = data
            .get_mut::<StreamTracks>()
            .expect("Could not get StreamTracks");
        let track = StreamTrack::new(channel_id, mixer_id, Platform::Mixer);
        // Remove from stream_tracks
        if stream_tracks.remove(&track) {
            let remaining_track_amount = stream_tracks
                .iter()
                .filter(|track| track.user_id == mixer_id)
                .count();
            // Remove from database
            let mysql = data.get::<MySQL>().expect("Could not get MySQL");
            if let Err(why) = mysql.remove_stream_track(channel_id, mixer_id, Platform::Mixer) {
                warn!("Error while removing stream track: {}", why);
            }
            if remaining_track_amount == 0 {
                // Remove from subscriptions
                let mixer = data.get_mut::<Mixer>().expect("Could not get Mixer");
                if let Err(why) = mixer.unsubscribe(mixer_id).await {
                    warn!("Error while unsubscribing channel {}: {}", mixer_id, why);
                }
                // Remove from HashMap
                let channels_lock = data
                    .get_mut::<MixerChannels>()
                    .expect("Could not get MixerChannels");
                let mut channels = channels_lock.write().await;
                channels.remove(&mixer_id);
            }
            return Ok(true);
        }
    }
    Ok(false)
}

async fn handle_twitch_remove(
    name: &str,
    channel: u64,
    data: &Arc<RwLock<TypeMap>>,
) -> Result<bool, String> {
    let twitch_id = {
        let data = data.read().await;
        let twitch_users = data
            .get::<TwitchUsers>()
            .expect("Could not get TwitchUsers");
        twitch_users.get(name).copied()
    };
    if let Some(twitch_id) = twitch_id {
        let mut data = data.write().await;
        let stream_tracks = data
            .get_mut::<StreamTracks>()
            .expect("Could not get StreamTracks");
        let track = StreamTrack::new(channel, twitch_id, Platform::Twitch);
        if stream_tracks.remove(&track) {
            let mysql = data.get::<MySQL>().expect("Could not get MySQL");
            if let Err(why) = mysql.remove_stream_track(channel, twitch_id, Platform::Twitch) {
                warn!("Error while removing stream track: {}", why);
            }
            return Ok(true);
        }
    }
    Ok(false)
}
