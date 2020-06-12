use super::{Pages, Pagination};

use crate::{embeds::RecentData, Error, Osu};

use rosu::models::{Beatmap, Score, User};
use serenity::{
    async_trait,
    cache::Cache,
    prelude::{RwLock, TypeMap},
};
use std::{collections::HashMap, sync::Arc};

pub struct RecentPagination {
    pages: Pages,
    user: Box<User>,
    scores: Vec<Score>,
    maps: HashMap<u32, Beatmap>,
    best: Vec<Score>,
    global: HashMap<u32, Vec<Score>>,
    cache: Arc<Cache>,
    data: Arc<RwLock<TypeMap>>,
}

impl RecentPagination {
    pub fn new(
        user: User,
        scores: Vec<Score>,
        maps: HashMap<u32, Beatmap>,
        best: Vec<Score>,
        global: HashMap<u32, Vec<Score>>,
        cache: Arc<Cache>,
        data: Arc<RwLock<TypeMap>>,
    ) -> Self {
        Self {
            pages: Pages::new(5, scores.len()),
            user: Box::new(user),
            scores,
            maps,
            best,
            global,
            cache,
            data,
        }
    }

    pub fn maps(self) -> HashMap<u32, Beatmap> {
        self.maps
    }
}

#[async_trait]
impl Pagination for RecentPagination {
    type PageData = RecentData;
    fn pages(&self) -> Pages {
        self.pages
    }
    fn pages_mut(&mut self) -> &mut Pages {
        &mut self.pages
    }
    async fn build_page(&mut self) -> Result<Self::PageData, Error> {
        let score = self.scores.get(self.index()).unwrap();
        let map_id = score.beatmap_id.unwrap();
        // Make sure map is ready
        #[allow(clippy::clippy::map_entry)]
        if !self.maps.contains_key(&map_id) {
            let data = self.data.read().await;
            let osu = data.get::<Osu>().unwrap();
            let map = score.get_beatmap(osu).await?;
            self.maps.insert(map_id, map);
        }
        let map = self.maps.get(&map_id).unwrap();
        // Make sure map leaderboard is ready
        #[allow(clippy::clippy::map_entry)]
        if !self.global.contains_key(&map.beatmap_id) {
            let data = self.data.read().await;
            let osu = data.get::<Osu>().unwrap();
            let global_lb = map.get_global_leaderboard(&osu, 50).await?;
            self.global.insert(map.beatmap_id, global_lb);
        };
        let global_lb = self.global.get(&map.beatmap_id).unwrap();
        // Create embed data
        RecentData::new(
            &*self.user,
            score,
            map,
            &self.best,
            &global_lb,
            (&self.cache, &self.data),
        )
        .await
    }
}
