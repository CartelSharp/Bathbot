use crate::embeds::EmbedData;

#[derive(Clone)]
pub struct BGStartEmbed {
    title: &'static str,
    description: &'static str,
}

impl BGStartEmbed {
    pub fn new() -> Self {
        let title = "React to include tag, unreact to exclude tag";
        let description = "(Not all backgrounds have been tagged properly yet, \
        I suggest to ✅ right away for now)\n\
        ```\n\
        🍋: Easy 🎨: Weeb 😱: Hard name 🗽: English 💯: Tech\n\
        🤓: Hard 🍨: Kpop 🪀: Alternate 🌀: Streams ✅: Lock in\n\
        🤡: Meme 👨‍🌾: Farm 🟦: Blue sky  👴: Old\n\
        ```";
        Self { title, description }
    }
}

impl EmbedData for BGStartEmbed {
    fn title(&self) -> Option<&str> {
        Some(self.title)
    }
    fn description(&self) -> Option<&str> {
        Some(self.description)
    }
}
