use super::{create_collector, Pages, Pagination};

use crate::embeds::TopEmbed;

use failure::Error;
use rosu::models::{Beatmap, GameMode, Score, User};
use serenity::{
    async_trait,
    cache::Cache,
    collector::ReactionCollector,
    model::{channel::Message, id::UserId},
    prelude::{Context, RwLock, TypeMap},
};
use std::sync::Arc;

pub struct TopPagination {
    msg: Message,
    collector: ReactionCollector,
    pages: Pages,
    user: User,
    scores: Vec<(usize, Score, Beatmap)>,
    mode: GameMode,
    cache: Arc<Cache>,
    data: Arc<RwLock<TypeMap>>,
}

impl TopPagination {
    pub async fn new(
        ctx: &Context,
        msg: Message,
        author: UserId,
        user: User,
        scores: Vec<(usize, Score, Beatmap)>,
        mode: GameMode,
    ) -> Self {
        let collector = create_collector(ctx, &msg, author, 90).await;
        let cache = Arc::clone(&ctx.cache);
        let data = Arc::clone(&ctx.data);
        Self {
            pages: Pages::new(5, scores.len()),
            msg,
            collector,
            user,
            scores,
            mode,
            cache,
            data,
        }
    }
}

#[async_trait]
impl Pagination for TopPagination {
    type PageData = TopEmbed;
    fn msg(&mut self) -> &mut Message {
        &mut self.msg
    }
    fn collector(&mut self) -> &mut ReactionCollector {
        &mut self.collector
    }
    fn pages(&self) -> Pages {
        self.pages
    }
    fn pages_mut(&mut self) -> &mut Pages {
        &mut self.pages
    }
    async fn build_page(&mut self) -> Result<Self::PageData, Error> {
        TopEmbed::new(
            &self.user,
            self.scores
                .iter()
                .skip(self.pages.index)
                .take(self.pages.per_page),
            self.mode,
            (self.page(), self.pages.total_pages),
            (&self.cache, &self.data),
        )
        .await
    }
}
