use crate::{
    embeds::{osu, Author, EmbedData},
    util::{globals::AVATAR_URL, numbers::round_and_comma, osu::pp_missing},
};

use rosu::models::{Score, User};

#[derive(Clone)]
pub struct RankEmbed {
    description: String,
    title: String,
    thumbnail: String,
    author: Author,
}

impl RankEmbed {
    pub fn new(
        user: User,
        scores: Vec<Score>,
        rank: usize,
        country: Option<String>,
        rank_holder: User,
    ) -> Self {
        let country = country.unwrap_or_else(|| '#'.to_string());
        let title = format!(
            "How many pp is {name} missing to reach rank {country}{rank}?",
            name = user.username,
            country = country,
            rank = rank
        );
        let description = if user.pp_raw > rank_holder.pp_raw {
            format!(
                "Rank {country}{rank} is currently held by {holder_name} with \
                 **{holder_pp}pp**, so {name} is with **{pp}pp** already above that.",
                country = country,
                rank = rank,
                holder_name = rank_holder.username,
                holder_pp = round_and_comma(rank_holder.pp_raw),
                name = user.username,
                pp = round_and_comma(user.pp_raw)
            )
        } else if scores.is_empty() {
            format!(
                "Rank {country}{rank} is currently held by {holder_name} with \
                 **{holder_pp}pp**, so {name} is missing **{holder_pp}** raw pp, \
                 achievable by a single score worth **{holder_pp}pp**.",
                country = country,
                rank = rank,
                holder_name = rank_holder.username,
                holder_pp = round_and_comma(rank_holder.pp_raw),
                name = user.username,
            )
        } else {
            let (required, _) = pp_missing(user.pp_raw, rank_holder.pp_raw, &scores);
            format!(
                "Rank {country}{rank} is currently held by {holder_name} with \
                 **{holder_pp}pp**, so {name} is missing **{missing}** raw pp, \
                 achievable by a single score worth **{pp}pp**.",
                country = country,
                rank = rank,
                holder_name = rank_holder.username,
                holder_pp = round_and_comma(rank_holder.pp_raw),
                name = user.username,
                missing = round_and_comma(rank_holder.pp_raw - user.pp_raw),
                pp = round_and_comma(required),
            )
        };
        Self {
            title,
            description,
            author: osu::get_user_author(&user),
            thumbnail: format!("{}{}", AVATAR_URL, user.user_id),
        }
    }
}

impl EmbedData for RankEmbed {
    fn description(&self) -> Option<&str> {
        Some(&self.description)
    }
    fn thumbnail(&self) -> Option<&str> {
        Some(&self.thumbnail)
    }
    fn author(&self) -> Option<&Author> {
        Some(&self.author)
    }
    fn title(&self) -> Option<&str> {
        Some(&self.title)
    }
}
