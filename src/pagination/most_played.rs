use super::{Pages, Pagination};

use crate::{embeds::BasicEmbedData, scraper::MostPlayedMap, Error};

use rosu::models::User;
use serenity::{
    async_trait,
    client::Context,
    collector::ReactionCollector,
    model::{channel::Message, id::UserId},
};

pub struct MostPlayedPagination {
    msg: Message,
    collector: ReactionCollector,
    pages: Pages,
    user: Box<User>,
    maps: Vec<MostPlayedMap>,
}

impl MostPlayedPagination {
    pub async fn new(
        ctx: &Context,
        msg: Message,
        author: UserId,
        user: User,
        maps: Vec<MostPlayedMap>,
    ) -> Self {
        let collector = Self::create_collector(ctx, &msg, author, 90).await;
        Self {
            msg,
            collector,
            pages: Pages::new(10, maps.len()),
            user: Box::new(user),
            maps,
        }
    }
}

#[async_trait]
impl Pagination for MostPlayedPagination {
    type PageData = BasicEmbedData;
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
        Ok(BasicEmbedData::create_mostplayed(
            &*self.user,
            self.maps.iter().skip(self.index()).take(self.per_page()),
            (self.page(), self.total_pages()),
        ))
    }
}