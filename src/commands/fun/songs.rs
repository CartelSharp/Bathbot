use crate::{
    util::{discord::add_guild, globals::GENERAL_ISSUE, MessageExt},
    Guilds,
};

use serenity::{
    framework::standard::{macros::command, CommandResult},
    model::prelude::Message,
    prelude::Context,
};
use tokio::time;

async fn song_send(lyrics: &[&str], delay: u64, ctx: &Context, msg: &Message) -> CommandResult {
    let allow = {
        if let Some(guild_id) = msg.guild_id {
            let contains_guild = {
                let data = ctx.data.read().await;
                let guilds = data.get::<Guilds>().unwrap();
                guilds.contains_key(&guild_id)
            };
            if !contains_guild {
                if let Err(why) = add_guild(ctx, guild_id).await {
                    msg.channel_id
                        .say(ctx, GENERAL_ISSUE)
                        .await?
                        .reaction_delete(ctx, msg.author.id)
                        .await;
                    return Err(why.into());
                }
            }
            let data = ctx.data.read().await;
            let guilds = data.get::<Guilds>().unwrap();
            guilds.get(&guild_id).unwrap().with_lyrics
        } else {
            true
        }
    };
    if allow {
        let mut interval = time::interval(time::Duration::from_millis(delay));
        interval.tick().await;
        for line in lyrics {
            msg.channel_id.say(ctx, format!("♫ {} ♫", line)).await?;
            interval.tick().await;
        }
    } else {
        msg.channel_id
            .say(
                ctx,
                "The server's big boys disabled song commands. \
                Server authorities can re-enable them by typing `<lyrics`",
            )
            .await?
            .reaction_delete(ctx, msg.author.id)
            .await;
    }
    Ok(())
}

#[command]
#[description = "Making me sing https://youtu.be/xpkkakkDhN4?t=65"]
#[bucket = "songs"]
pub async fn bombsaway(ctx: &Context, msg: &Message) -> CommandResult {
    let lyrics = &[
        "Tick tick tock and it's bombs awayyyy",
        "Come ooon, it's the only way",
        "Save your-self for a better dayyyy",
        "No, no, we are falling dooo-ooo-ooo-ooown",
        "I know, you know - this is over",
        "Tick tick tock and it's bombs awayyyy",
        "Now we're falling -- now we're falling doooown",
    ];
    song_send(lyrics, 2500, ctx, msg).await
}

#[command]
#[description = "Making me sing https://youtu.be/BjFWk0ncr70?t=12"]
#[bucket = "songs"]
pub async fn catchit(ctx: &Context, msg: &Message) -> CommandResult {
    let lyrics = &[
        "This song is one you won't forget",
        "It will get stuck -- in your head",
        "If it does, then you can't blame me",
        "Just like I said - too catchy",
    ];
    song_send(lyrics, 3000, ctx, msg).await
}

#[command]
#[description = "Making me sing https://youtu.be/_yWU0lFghxU?t=54"]
#[bucket = "songs"]
pub async fn ding(ctx: &Context, msg: &Message) -> CommandResult {
    let lyrics = &[
        "Oh-oh-oh, hübsches Ding",
        "Ich versteck' mein' Ehering",
        "Klinglingeling, wir könnten's bring'n",
        "Doch wir nuckeln nur am Drink",
        "Oh-oh-oh, hübsches Ding",
        "Du bist Queen und ich bin King",
        "Wenn ich dich seh', dann muss ich sing'n:",
        "Tingalingaling, you pretty thing!",
    ];
    song_send(lyrics, 2500, ctx, msg).await
}

#[command]
#[description = "Making me sing https://youtu.be/0jgrCKhxE1s?t=77"]
#[bucket = "songs"]
pub async fn fireandflames(ctx: &Context, msg: &Message) -> CommandResult {
    let lyrics = &[
        "So far away we wait for the day-yay",
        "For the lives all so wasted and gooone",
        "We feel the pain of a lifetime lost in a thousand days",
        "Through the fire and the flames we carry ooooooon",
    ];
    song_send(lyrics, 3000, ctx, msg).await
}

#[command]
#[description = "Making me sing https://youtu.be/psuRGfAaju4?t=25"]
#[bucket = "songs"]
pub async fn fireflies(ctx: &Context, msg: &Message) -> CommandResult {
    let lyrics = &[
        "You would not believe your eyes",
        "If ten million fireflies",
        "Lit up the world as I fell asleep",
        "'Cause they'd fill the open air",
        "And leave teardrops everywhere",
        "You'd think me rude, but I would just stand and -- stare",
    ];
    song_send(lyrics, 2500, ctx, msg).await
}

#[command]
#[description = "Making me sing https://youtu.be/la9C0n7jSsI"]
#[bucket = "songs"]
pub async fn flamingo(ctx: &Context, msg: &Message) -> CommandResult {
    let lyrics = &[
        "How many shrimps do you have to eat",
        "before you make your skin turn pink?",
        "Eat too much and you'll get sick",
        "Shrimps are pretty rich",
    ];
    song_send(lyrics, 2500, ctx, msg).await
}

#[command]
#[description = "Making me sing https://youtu.be/SyJMQg3spck?t=43"]
#[bucket = "songs"]
pub async fn pretender(ctx: &Context, msg: &Message) -> CommandResult {
    let lyrics = &[
        "What if I say I'm not like the others?",
        "What if I say I'm not just another oooone of your plays?",
        "You're the pretender",
        "What if I say that I will never surrender?",
    ];
    song_send(lyrics, 3000, ctx, msg).await
}

#[command]
#[description = "Making me sing https://youtu.be/hjGZLnja1o8?t=41"]
#[bucket = "songs"]
#[aliases("1273")]
pub async fn rockefeller(ctx: &Context, msg: &Message) -> CommandResult {
    let lyrics = &[
        "1 - 2 - 7 - 3",
        "down the Rockefeller street.",
        "Life is marchin' on, do you feel that?",
        "1 - 2 - 7 - 3",
        "down the Rockefeller street.",
        "Everything is more than surreal",
    ];
    song_send(lyrics, 2500, ctx, msg).await
}

#[command]
#[description = "Making me sing https://youtu.be/DT6tpUbWOms?t=47"]
#[bucket = "songs"]
pub async fn tijdmachine(ctx: &Context, msg: &Message) -> CommandResult {
    let lyrics = &[
        "Als ik denk aan al die dagen,",
        "dat ik mij zo heb misdragen.",
        "Dan denk ik, - had ik maar een tijdmachine -- tijdmachine",
        "Maar die heb ik niet,",
        "dus zal ik mij gedragen,",
        "en zal ik blijven sparen,",
        "sparen voor een tijjjdmaaachine.",
    ];
    song_send(lyrics, 2500, ctx, msg).await
}
