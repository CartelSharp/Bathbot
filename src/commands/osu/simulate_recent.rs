use crate::{
    commands::osu::MINIMIZE_DELAY,
    database::MySQL,
    messages::{BotEmbed, SimulateData},
    util::globals::OSU_API_ISSUE,
    DiscordLinks, Osu,
};

use rosu::{
    backend::requests::RecentRequest,
    models::{
        ApprovalStatus::{Loved, Ranked},
        GameMode,
    },
};
use serenity::{
    framework::standard::{macros::command, Args, CommandError, CommandResult},
    model::prelude::Message,
    prelude::Context,
};
use std::{error::Error, thread};
use tokio::runtime::Runtime;

fn simulate_recent_send(
    mode: GameMode,
    ctx: &mut Context,
    msg: &Message,
    mut args: Args,
) -> CommandResult {
    let name: String = if args.is_empty() {
        let data = ctx.data.read();
        let links = data
            .get::<DiscordLinks>()
            .expect("Could not get DiscordLinks");
        match links.get(msg.author.id.as_u64()) {
            Some(name) => name.clone(),
            None => {
                msg.channel_id.say(
                    &ctx.http,
                    "Either specify an osu name or link your discord \
                     to an osu profile via `<link osuname`",
                )?;
                return Ok(());
            }
        }
    } else {
        args.single_quoted()?
    };
    let mut rt = Runtime::new().unwrap();

    // Retrieve the recent score
    let score = {
        let request = RecentRequest::with_username(&name).mode(mode).limit(1);
        let data = ctx.data.read();
        let osu = data.get::<Osu>().expect("Could not get osu client");
        let mut scores = match rt.block_on(request.queue(osu)) {
            Ok(scores) => scores,
            Err(why) => {
                msg.channel_id.say(&ctx.http, OSU_API_ISSUE)?;
                return Err(CommandError::from(why.to_string()));
            }
        };
        match scores.pop() {
            Some(score) => score,
            None => {
                msg.channel_id.say(
                    &ctx.http,
                    format!("No recent plays found for user `{}`", name),
                )?;
                return Ok(());
            }
        }
    };

    // Retrieving the score's beatmap
    let (map_to_db, map) = {
        let data = ctx.data.read();
        let mysql = data.get::<MySQL>().expect("Could not get MySQL");
        match mysql.get_beatmap(score.beatmap_id.unwrap()) {
            Ok(map) => (false, map),
            Err(_) => {
                let osu = data.get::<Osu>().expect("Could not get osu client");
                let map = match rt.block_on(score.get_beatmap(osu)) {
                    Ok(m) => m,
                    Err(why) => {
                        msg.channel_id.say(&ctx.http, OSU_API_ISSUE)?;
                        return Err(CommandError::from(why.to_string()));
                    }
                };
                (
                    map.approval_status == Ranked || map.approval_status == Loved,
                    map,
                )
            }
        }
    };

    // Accumulate all necessary data
    let map_copy = if map_to_db { Some(map.clone()) } else { None };
    let data = match SimulateData::new(Some(score), map, mode, ctx.cache.clone()) {
        Ok(data) => data,
        Err(why) => {
            msg.channel_id.say(
                &ctx.http,
                "Some issue while calculating simulaterecent data, blame bade",
            )?;
            return Err(CommandError::from(why.description()));
        }
    };

    // Creating the embed
    let embed = BotEmbed::SimulateScore(&data);
    let mut msg = msg.channel_id.send_message(&ctx.http, |m| {
        m.content("Simulated score:").embed(|e| embed.create(e))
    })?;

    // Add map to database if its not in already
    if let Some(map) = map_copy {
        let data = ctx.data.read();
        let mysql = data.get::<MySQL>().expect("Could not get MySQL");
        if let Err(why) = mysql.insert_beatmap(&map) {
            warn!(
                "Could not add map of simulaterecent command to database: {}",
                why
            );
        }
    }

    // Minimize embed after delay
    let embed = BotEmbed::SimulateScoreMini(Box::new(data));
    msg.edit(&ctx, |m| {
        thread::sleep(MINIMIZE_DELAY);
        m.embed(|e| embed.create(e))
    })?;
    Ok(())
}

#[command]
#[description = "Display an unchoked version of user's most recent play"]
#[usage = "badewanne3"]
#[aliases("sr")]
pub fn simulaterecent(ctx: &mut Context, msg: &Message, args: Args) -> CommandResult {
    simulate_recent_send(GameMode::STD, ctx, msg, args)
}

/*
// TODO
#[command]
#[description = "Display an unchoked version of user's most recent mania play"]
#[usage = "badewanne3"]
#[aliases("srm")]
pub fn simulaterecentmania(ctx: &mut Context, msg: &Message, args: Args) -> CommandResult {
    simulate_recent_send(GameMode::MNA, ctx, msg, args)
}
*/