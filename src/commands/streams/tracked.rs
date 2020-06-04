use crate::{database::Platform, util::discord, MixerChannels, StreamTracks, TwitchUsers};

use rayon::prelude::*;
use serenity::{
    framework::standard::{macros::command, CommandResult},
    model::prelude::Message,
    prelude::Context,
};

#[command]
#[description = "List all streams that are tracked in this channel"]
#[aliases("tracked")]
async fn trackedstreams(ctx: &Context, msg: &Message) -> CommandResult {
    // Twitch
    let mut twitch_users: Vec<_> = {
        let data = ctx.data.read().await;
        let twitch_users = data
            .get::<TwitchUsers>()
            .expect("Could not get TwitchUsers");
        let tracks = data
            .get::<StreamTracks>()
            .expect("Could not get StreamTracks");
        twitch_users
            .par_iter()
            .filter(|(_, &twitch_id)| {
                tracks.iter().any(|track| {
                    track.user_id == twitch_id
                        && track.channel_id == msg.channel_id.0
                        && track.platform == Platform::Twitch
                })
            })
            .map(|(name, _)| name.clone())
            .collect()
    };
    twitch_users.sort();
    let twitch_str = if twitch_users.is_empty() {
        "None".to_string()
    } else {
        twitch_users.join("`, `")
    };

    // Mixer
    let mut mixer_users: Vec<_> = {
        let data = ctx.data.read().await;
        let mixer_channels_lock = data
            .get::<MixerChannels>()
            .expect("Could not get MixerChannels");
        let mixer_channels = mixer_channels_lock.read().await;
        let tracks = data
            .get::<StreamTracks>()
            .expect("Could not get StreamTracks");
        mixer_channels
            .par_iter()
            .filter(|(&id, _)| {
                tracks.iter().any(|track| {
                    track.user_id == id
                        && track.channel_id == msg.channel_id.0
                        && track.platform == Platform::Mixer
                })
            })
            .map(|(_, channel)| channel.name.clone())
            .collect()
    };
    mixer_users.sort();
    let mixer_str = if mixer_users.is_empty() {
        "None".to_string()
    } else {
        mixer_users.join("`, `")
    };

    // Sending the msg
    let response = msg
        .channel_id
        .say(
            &ctx.http,
            format!(
                "Tracked streams in this channel:\n\
                Twitch: `{}`\n\
                Mixer: `{}`",
                twitch_str, mixer_str
            ),
        )
        .await?;
    discord::reaction_deletion(&ctx, response, msg.author.id).await;
    Ok(())
}
