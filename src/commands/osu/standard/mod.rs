pub use crate::commands::osu::recent::*;

use serenity::framework::standard::macros::group;

#[group]
#[description = "Commands for osu!'s standard mode"]
#[commands(recent)]
pub struct Osu;
