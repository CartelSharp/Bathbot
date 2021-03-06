pub mod common;
pub mod fruits;
mod leaderboard;
mod link;
pub mod mania;
mod map;
mod match_costs;
mod most_played;
mod most_played_common;
pub mod osustats_globals;
pub mod pp;
pub mod profile;
pub mod rank;
pub mod recent;
pub mod recent_lb;
mod scores;
mod simulate;
pub mod simulate_recent;
pub mod standard;
pub mod taiko;
pub mod top;
pub mod whatif;

pub use self::fruits::*;
pub use self::mania::*;
pub use self::standard::*;
pub use self::taiko::*;
pub use leaderboard::*;
pub use link::*;
pub use map::*;
pub use match_costs::*;
pub use most_played::*;
pub use most_played_common::*;
pub use scores::*;
pub use simulate::*;

use serenity::framework::standard::macros::group;

#[group]
#[description = "Commands for all osu! modes"]
#[commands(
    link,
    scores,
    simulate,
    map,
    matchcosts,
    mostplayed,
    mostplayedcommon,
    leaderboard,
    globalleaderboard
)]
struct OsuGeneral;
