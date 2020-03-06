use crate::arguments::{self, ModSelection};

use rosu::models::{GameMods, Grade};
use serenity::framework::standard::Args;

pub struct TopArgs {
    pub name: Option<String>,
    pub mods: Option<(GameMods, ModSelection)>,
    pub acc: Option<f32>,
    pub combo: Option<u32>,
    pub grade: Option<Grade>,
}

impl TopArgs {
    pub fn new(mut args: Args) -> Result<Self, String> {
        let mut args = arguments::first_n(&mut args, 8);
        let acc = arguments::acc(&mut args)?;
        let combo = arguments::combo(&mut args)?;
        let grade = arguments::grade(&mut args)?;
        let mods = arguments::mods(&mut args);
        let name = args.into_iter().rev().next();
        Ok(Self {
            name,
            mods,
            acc,
            combo,
            grade,
        })
    }
}