use crate::{
    embeds::{osu, Author, EmbedData, Footer},
    util::{
        datetime::how_long_ago,
        discord::CacheData,
        globals::{AVATAR_URL, HOMEPAGE},
        numbers::with_comma_u64,
        osu::grade_emote,
        pp::{Calculations, PPCalculator},
    },
};

use failure::Error;
use rosu::models::{Beatmap, GameMode, Score, User};
use std::{fmt::Write, sync::Arc};

#[derive(Clone)]
pub struct TopEmbed {
    description: String,
    author: Author,
    thumbnail: String,
    footer: Footer,
}

impl TopEmbed {
    pub async fn new<'i, S, D>(
        user: &User,
        scores_data: S,
        mode: GameMode,
        pages: (usize, usize),
        cache_data: D,
    ) -> Result<Self, Error>
    where
        S: Iterator<Item = &'i (usize, Score, Beatmap)>,
        D: CacheData,
    {
        let mut description = String::with_capacity(512);
        for (idx, score, map) in scores_data {
            let grade = { grade_emote(score.grade, cache_data.cache()).await };
            let calculations = Calculations::PP | Calculations::MAX_PP | Calculations::STARS;
            let mut calculator = PPCalculator::new()
                .score(score)
                .map(map)
                .data(Arc::clone(cache_data.data()));
            calculator.calculate(calculations).await?;
            let stars = osu::get_stars(calculator.stars().unwrap());
            let pp = osu::get_pp(calculator.pp(), calculator.max_pp());
            let _ = writeln!(
                description,
                "**{idx}. [{title} [{version}]]({base}b/{id}) {mods}** [{stars}]\n\
                {grade} {pp} ~ ({acc}) ~ {score}\n[ {combo} ] ~ {hits} ~ {ago}",
                idx = idx,
                title = map.title,
                version = map.version,
                base = HOMEPAGE,
                id = map.beatmap_id,
                mods = osu::get_mods(score.enabled_mods),
                stars = stars,
                grade = grade,
                pp = pp,
                acc = osu::get_acc(&score, mode),
                score = with_comma_u64(score.score as u64),
                combo = osu::get_combo(&score, &map),
                hits = osu::get_hits(score, mode),
                ago = how_long_ago(&score.date)
            );
        }
        description.pop();
        Ok(Self {
            thumbnail: format!("{}{}", AVATAR_URL, user.user_id),
            description,
            author: osu::get_user_author(user),
            footer: Footer::new(format!("Page {}/{}", pages.0, pages.1)),
        })
    }
}

impl EmbedData for TopEmbed {
    fn description(&self) -> Option<&str> {
        Some(&self.description)
    }
    fn thumbnail(&self) -> Option<&str> {
        Some(&self.thumbnail)
    }
    fn author(&self) -> Option<&Author> {
        Some(&self.author)
    }
    fn footer(&self) -> Option<&Footer> {
        Some(&self.footer)
    }
}
