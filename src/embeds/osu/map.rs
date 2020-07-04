use crate::{
    embeds::{Author, EmbedData, Footer},
    roppai::Oppai,
    util::{
        datetime::sec_to_minsec,
        globals::{AVATAR_URL, HOMEPAGE, MAP_THUMB_URL},
        numbers::round,
        osu::prepare_beatmap_file,
        pp::{Calculations, PPCalculator},
    },
};

use chrono::{DateTime, Utc};
use failure::Error;
use rosu::models::{Beatmap, GameMode, GameMods};
use serenity::prelude::{RwLock, TypeMap};
use std::{fmt::Write, sync::Arc};

#[derive(Clone)]
pub struct MapEmbed {
    title: String,
    url: String,
    thumbnail: Option<String>,
    footer: Footer,
    author: Author,
    image: Option<&'static str>,
    timestamp: DateTime<Utc>,
    fields: Vec<(String, String, bool)>,
}

impl MapEmbed {
    pub async fn new(
        map: &Beatmap,
        mods: GameMods,
        with_thumbnail: bool,
        pages: (usize, usize),
        data: Arc<RwLock<TypeMap>>,
    ) -> Result<Self, Error> {
        let mut title = String::with_capacity(32);
        if map.mode == GameMode::MNA {
            let _ = write!(title, "[{}K] ", map.diff_cs as u32);
        }
        let _ = write!(title, "{} - {}", map.artist, map.title);
        let mut ar = map.diff_ar;
        let mut od = map.diff_od;
        let mut hp = map.diff_hp;
        let mut cs = map.diff_cs;
        let (pp, stars) = match map.mode {
            GameMode::STD | GameMode::TKO => {
                // Prepare oppai
                let map_path = prepare_beatmap_file(map.beatmap_id).await?;
                let mut oppai = Oppai::new();
                oppai.set_mods(mods.bits()).calculate(&map_path)?;
                ar = oppai.get_ar();
                od = oppai.get_od();
                hp = oppai.get_hp();
                cs = oppai.get_cs();
                let pp = oppai.get_pp();
                let stars = oppai.get_stars();
                (pp, stars)
            }
            GameMode::MNA | GameMode::CTB => {
                let calculations = Calculations::MAX_PP | Calculations::STARS;
                let mut calculator = PPCalculator::new().map(map).data(Arc::clone(&data));
                if let Err(why) = calculator.calculate(calculations).await {
                    warn!("Error while calculating pp for <map: {}", why);
                }
                (
                    calculator.max_pp().unwrap_or_default(),
                    calculator.stars().unwrap_or_default(),
                )
            }
        };
        let thumbnail = if with_thumbnail {
            Some(format!("{}{}l.jpg", MAP_THUMB_URL, map.beatmapset_id))
        } else {
            None
        };
        let image = if with_thumbnail {
            None
        } else {
            Some("attachment://map_graph.png")
        };
        let mut info_value = String::with_capacity(128);
        let _ = write!(info_value, "Max PP: `{}`", round(pp));
        if let Some(combo) = map.max_combo {
            let _ = write!(info_value, " Combo: `{}x`", combo);
        }
        let _ = writeln!(info_value, " Stars: `{}★`", round(stars));
        let mut seconds_total = map.seconds_total;
        let mut seconds_drain = map.seconds_drain;
        let mut bpm = map.bpm;
        if mods.contains(GameMods::DoubleTime) {
            seconds_total = (seconds_total as f32 * 2.0 / 3.0) as u32;
            seconds_drain = (seconds_drain as f32 * 2.0 / 3.0) as u32;
            bpm *= 1.5;
        } else if mods.contains(GameMods::HalfTime) {
            seconds_total = (seconds_total as f32 * 3.0 / 2.0) as u32;
            seconds_drain = (seconds_drain as f32 * 3.0 / 2.0) as u32;
            bpm /= 1.5;
        }
        let _ = write!(
            info_value,
            "Length: `{}` (`{}`) BPM: `{}` Objects: `{}`\n\
            CS: `{}` AR: `{}` OD: `{}` HP: `{}` Spinners: `{}`",
            sec_to_minsec(seconds_total),
            sec_to_minsec(seconds_drain),
            bpm,
            map.count_objects(),
            round(cs),
            round(ar),
            round(od),
            round(hp),
            map.count_spinner,
        );
        let mut info_name = format!("__[{}]__", map.version);
        if !mods.is_empty() {
            let _ = write!(info_name, " +{}", mods);
        }
        let fields = vec![
            (info_name, info_value, true),
            (
                "Download".to_owned(),
                format!(
                    "[Mapset]({base}d/{mapset_id})\n\
                    [No Video]({base}d/{mapset_id}n)\n\
                    [Bloodcat](https://bloodcat.com/osu/s/{mapset_id})\n\
                    <osu://dl/{mapset_id}>",
                    base = HOMEPAGE,
                    mapset_id = map.beatmapset_id
                ),
                true,
            ),
            (
                format!(
                    "osu!{}  :heart: {}  :play_pause: {}",
                    match map.mode {
                        GameMode::STD => "standard",
                        GameMode::TKO => "taiko",
                        GameMode::CTB => "fruits",
                        GameMode::MNA => "mania",
                    },
                    map.favourite_count,
                    map.playcount
                ),
                format!("{:?}, {:?}", map.language, map.genre),
                false,
            ),
        ];
        let (date_text, timestamp) = if let Some(approved_date) = map.approved_date {
            (format!("{:?}", map.approval_status), approved_date)
        } else {
            ("Last updated".to_owned(), map.last_update)
        };
        let author = Author::new(format!("Created by {}", map.creator))
            .url(format!("{}u/{}", HOMEPAGE, map.creator_id))
            .icon_url(format!("{}{}", AVATAR_URL, map.creator_id));
        let footer_text = format!(
            "Map {} out of {} in the mapset, {}",
            pages.0, pages.1, date_text
        );
        let footer = Footer::new(footer_text);
        Ok(Self {
            title,
            image,
            footer,
            fields,
            author,
            thumbnail,
            timestamp,
            url: format!("{}b/{}", HOMEPAGE, map.beatmap_id),
        })
    }
}

impl EmbedData for MapEmbed {
    fn thumbnail(&self) -> Option<&str> {
        self.thumbnail.as_deref()
    }
    fn title(&self) -> Option<&str> {
        Some(&self.title)
    }
    fn url(&self) -> Option<&str> {
        Some(&self.url)
    }
    fn image(&self) -> Option<&str> {
        self.image
    }
    fn footer(&self) -> Option<&Footer> {
        Some(&self.footer)
    }
    fn author(&self) -> Option<&Author> {
        Some(&self.author)
    }
    fn fields(&self) -> Option<Vec<(String, String, bool)>> {
        Some(self.fields.clone())
    }
    fn timestamp(&self) -> Option<&DateTime<Utc>> {
        Some(&self.timestamp)
    }
}
