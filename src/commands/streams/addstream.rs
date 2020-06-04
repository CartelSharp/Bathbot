use crate::{
    commands::checks::*,
    database::{Platform, StreamTrack},
    util::discord,
    Mixer, MixerChannels, MySQL, StreamTracks, Twitch, TwitchUsers,
};

use serenity::{
    framework::standard::{macros::command, Args, CommandResult},
    model::prelude::Message,
    prelude::{Context, RwLock, TypeMap},
};
use std::sync::Arc;

#[command]
#[checks(Authority)]
#[description = "Let me notify this channel whenever the given stream comes online"]
#[aliases("streamadd")]
#[usage = "[twitch / mixer] [stream name]"]
#[example = "twitch loltyler1"]
async fn addstream(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
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
    let result = match platform {
        Platform::Mixer => handle_mixer(&name, msg.channel_id.0, &ctx.data).await,
        Platform::Twitch => handle_twitch(&name, msg.channel_id.0, &ctx.data).await,
    };

    // Check if there was an issue
    if let Err(err_msg) = result {
        let response = msg.channel_id.say(&ctx.http, err_msg).await?;
        discord::reaction_deletion(ctx, response, msg.author.id).await;
        return Ok(());
    }

    // Sending the msg
    let content = format!(
        "I'm now tracking `{}`'s {:?} stream in this channel",
        name, platform
    );
    let response = msg.channel_id.say(ctx, content).await?;
    discord::reaction_deletion(ctx, response, msg.author.id).await;
    Ok(())
}

async fn handle_mixer(
    name: &str,
    channel_id: u64,
    data: &Arc<RwLock<TypeMap>>,
) -> Result<(), String> {
    // Is channel already tracked somewhere?
    let mut mixer_id = {
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

    // If not, request channel
    if mixer_id.is_none() {
        let mut data = data.write().await;
        let mixer = data.get_mut::<Mixer>().expect("Could not get Mixer");
        match mixer.channel(name).await {
            Ok(mixer_channel) => {
                mixer_id = Some(mixer_channel.id);
                if let Err(why) = mixer.track_stream(mixer_channel.id).await {
                    warn!(
                        "Error while starting to track {}: {}",
                        mixer_channel.id, why
                    );
                    return Err("Error while starting to track, blame bade".to_string());
                }
            }
            Err(why) => {
                warn!("Error while retrieving Mixer channel: {}", why);
                return Err(
                    "Error while retrieving mixer channel. Is that the correct channel name?"
                        .to_string(),
                );
            }
        };
    }
    let mixer_id = mixer_id.unwrap();
    let track = StreamTrack::new(channel_id, mixer_id, Platform::Mixer);
    let mut data = data.write().await;
    let stream_tracks = data
        .get_mut::<StreamTracks>()
        .expect("Could not get StreamTracks");

    // If mixer user is not yet tracked in the channel, add entry to DB
    if stream_tracks.insert(track) {
        let mysql = data.get::<MySQL>().expect("Could not get MySQL");
        match mysql.add_stream_track(channel_id, mixer_id, Platform::Mixer) {
            Ok(_) => debug!("Inserted into stream_tracks table"),
            Err(why) => warn!("Error while adding stream track: {}", why),
        }
    }
    Ok(())
}

async fn handle_twitch(
    name: &str,
    channel: u64,
    data: &Arc<RwLock<TypeMap>>,
) -> Result<(), String> {
    let (twitch_id, insert) = {
        let data = data.read().await;
        let twitch_users = data
            .get::<TwitchUsers>()
            .expect("Could not get TwitchUsers");
        if twitch_users.contains_key(name) {
            (*twitch_users.get(name).unwrap(), false)
        } else {
            let twitch = data.get::<Twitch>().expect("Could not get Twitch");
            let twitch_id = match twitch.get_user(&name).await {
                Ok(user) => user.user_id,
                Err(_) => {
                    return Err(format!("Twitch user `{}` was not found", name));
                }
            };
            let mysql = data.get::<MySQL>().expect("Could not get MySQL");
            match mysql.add_twitch_user(twitch_id, &name) {
                Ok(_) => debug!("Inserted into twitch_users table"),
                Err(why) => warn!("Error while adding twitch user: {}", why),
            }
            (twitch_id, true)
        }
    };
    let mut data = data.write().await;
    if insert {
        let twitch_users = data
            .get_mut::<TwitchUsers>()
            .expect("Could not get TwitchUsers");
        twitch_users.insert(name.to_owned(), twitch_id);
    }
    let stream_tracks = data
        .get_mut::<StreamTracks>()
        .expect("Could not get StreamTracks");
    let track = StreamTrack::new(channel, twitch_id, Platform::Twitch);
    if stream_tracks.insert(track) {
        let mysql = data.get::<MySQL>().expect("Could not get MySQL");
        match mysql.add_stream_track(channel, twitch_id, Platform::Twitch) {
            Ok(_) => debug!("Inserted into stream_tracks table"),
            Err(why) => warn!("Error while adding stream track: {}", why),
        }
    }
    Ok(())
}
