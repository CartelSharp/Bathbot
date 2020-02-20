use crate::{
    database::MySQL,
    messages::{BotEmbed, CommonData},
    util::globals::OSU_API_ISSUE,
    DiscordLinks, Osu,
};

use itertools::Itertools;
use rosu::{
    backend::requests::{BeatmapRequest, UserRequest},
    models::{Beatmap, GameMode, Score, User},
};
use serenity::{
    framework::standard::{macros::command, Args, CommandError, CommandResult},
    model::prelude::Message,
    prelude::Context,
};
use std::{
    collections::{HashMap, HashSet},
    convert::From,
};
use tokio::runtime::Runtime;

fn common_send(mode: GameMode, ctx: &mut Context, msg: &Message, mut args: Args) -> CommandResult {
    // Parse the names
    if args.is_empty() {
        msg.channel_id.say(
            &ctx.http,
            "You need to specify at least one osu username. \
             If you're not linked, you must specify at least two names.",
        )?;
        return Ok(());
    }
    let mut names = Vec::with_capacity(args.len());
    while !args.is_empty() {
        names.push(args.trimmed().single_quoted::<String>()?);
    }
    if names.len() == 1 {
        let data = ctx.data.read();
        let links = data
            .get::<DiscordLinks>()
            .expect("Could not get DiscordLinks");
        match links.get(msg.author.id.as_u64()) {
            Some(name) => {
                names.push(name.clone());
            }
            None => {
                msg.channel_id.say(
                    &ctx.http,
                    "You need to specify at least one osu username. \
                     If you're not linked, you must specify at least two names.",
                )?;
                return Ok(());
            }
        }
    }
    let mut rt = Runtime::new().unwrap();

    // Retrieve all users and their top scores
    let requests: HashMap<String, UserRequest> = names
        .iter()
        .map(|name| (name.clone(), UserRequest::with_username(name).mode(mode)))
        .collect();
    let (users, mut all_scores): (HashMap<u32, User>, Vec<Vec<Score>>) = {
        let data = ctx.data.read();
        let osu = data.get::<Osu>().expect("Could not get osu client");
        let mut users = HashMap::with_capacity(requests.len());
        let mut all_scores = Vec::with_capacity(requests.len());
        for (name, request) in requests.into_iter() {
            let user = match rt.block_on(request.queue_single(&osu)) {
                Ok(result) => match result {
                    Some(user) => user,
                    None => {
                        msg.channel_id
                            .say(&ctx.http, format!("User `{}` was not found", name))?;
                        return Ok(());
                    }
                },
                Err(why) => {
                    msg.channel_id.say(&ctx.http, OSU_API_ISSUE)?;
                    return Err(CommandError::from(why.to_string()));
                }
            };
            let scores = match rt.block_on(user.get_top_scores(&osu, 100, mode)) {
                Ok(scores) => scores,
                Err(why) => {
                    msg.channel_id.say(&ctx.http, OSU_API_ISSUE)?;
                    return Err(CommandError::from(why.to_string()));
                }
            };
            users.insert(user.user_id, user);
            all_scores.push(scores);
        }
        (users, all_scores)
    };

    // Consider only scores on common maps
    let mut map_ids: HashSet<u32> = all_scores
        .iter()
        .map(|scores| {
            scores
                .iter()
                .map(|s| s.beatmap_id.unwrap())
                .collect::<HashSet<u32>>()
        })
        .flatten()
        .collect();
    map_ids.retain(|&id| {
        all_scores
            .iter()
            .all(|scores| scores.iter().any(|s| s.beatmap_id.unwrap() == id))
    });
    all_scores
        .iter_mut()
        .for_each(|scores| scores.retain(|s| map_ids.contains(&s.beatmap_id.unwrap())));

    // Try retrieving all maps of common scores from the database
    let mut maps: HashMap<u32, Beatmap> = {
        let map_ids: Vec<u32> = map_ids.iter().copied().collect();
        let data = ctx.data.read();
        let mysql = data.get::<MySQL>().expect("Could not get MySQL");
        mysql
            .get_beatmaps(&map_ids)
            .unwrap_or_else(|_| HashMap::default())
    };
    let amount_common = map_ids.len();
    info!(
        "Found {}/{} beatmaps in the database",
        maps.len(),
        amount_common
    );
    map_ids.retain(|id| !maps.contains_key(id));

    // Retrieve all missing maps from the API
    let missing_maps = if !map_ids.is_empty() {
        let data = ctx.data.read();
        let osu = data.get::<Osu>().expect("Could not get osu client");
        let mut missing_maps = Vec::with_capacity(map_ids.len());
        for id in map_ids {
            let req = BeatmapRequest::new().map_id(id);
            let map = match rt.block_on(req.queue_single(&osu)) {
                Ok(result) => match result {
                    Some(map) => {
                        maps.insert(map.beatmap_id, map.clone());
                        map
                    }
                    None => {
                        msg.channel_id
                            .say(&ctx.http, "Unexpected response from the API, blame bade")?;
                        return Ok(());
                    }
                },
                Err(why) => {
                    msg.channel_id.say(&ctx.http, OSU_API_ISSUE)?;
                    return Err(CommandError::from(why.to_string()));
                }
            };
            missing_maps.push(map);
        }
        Some(missing_maps)
    } else {
        None
    };

    // Accumulate all necessary data
    let names_join = names
        .chunks(names.len() - 1)
        .map(|chunk| chunk.join("`, `"))
        .join("` and `");
    let mut content = format!("`{}`", names_join);
    if amount_common == 0 {
        content.push_str(" have no common scores");
    } else {
        content.push_str(&format!(
            " have {} common beatmaps in their top 100",
            amount_common
        ));
        if amount_common > 10 {
            content.push_str(", here's the top 10 of them:");
        } else {
            content.push(':');
        }
    }
    let (data, thumbnail) = CommonData::new(users, all_scores, maps);

    // Creating the embed
    let embed = BotEmbed::UserCommonScores(data);
    let _ = msg.channel_id.send_message(&ctx.http, |m| {
        if !thumbnail.is_empty() {
            let bytes: &[u8] = &thumbnail;
            m.add_file((bytes, "avatar_fuse.png"));
        }
        m.content(content).embed(|e| embed.create(e))
    });

    // Add missing maps to database
    if let Some(maps) = missing_maps {
        let data = ctx.data.read();
        let mysql = data.get::<MySQL>().expect("Could not get MySQL");
        if let Err(why) = mysql.insert_beatmaps(maps) {
            warn!(
                "Could not add missing maps of common command to database: {}",
                why
            );
        }
    }
    Ok(())
}

#[command]
#[description = "Compare the users' top 100 and check which maps appear in each top list"]
#[usage = "badewanne3 \"nathan on osu\" idke"]
pub fn common(ctx: &mut Context, msg: &Message, args: Args) -> CommandResult {
    common_send(GameMode::STD, ctx, msg, args)
}

#[command]
#[description = "Compare the mania users' top 100 and check which maps appear in each top list"]
#[usage = "badewanne3 \"nathan on osu\" idke"]
#[aliases("commonm")]
pub fn commonmania(ctx: &mut Context, msg: &Message, args: Args) -> CommandResult {
    common_send(GameMode::MNA, ctx, msg, args)
}

#[command]
#[description = "Compare the taiko users' top 100 and check which maps appear in each top list"]
#[usage = "badewanne3 \"nathan on osu\" idke"]
#[aliases("commont")]
pub fn commontaiko(ctx: &mut Context, msg: &Message, args: Args) -> CommandResult {
    common_send(GameMode::TKO, ctx, msg, args)
}

#[command]
#[description = "Compare the ctb users' top 100 and check which maps appear in each top list"]
#[usage = "badewanne3 \"nathan on osu\" idke"]
#[aliases("commonc")]
pub fn commonctb(ctx: &mut Context, msg: &Message, args: Args) -> CommandResult {
    common_send(GameMode::CTB, ctx, msg, args)
}