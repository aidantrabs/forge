use crate::config::SiteConfig;
use crate::content::Post;
use rss::{ChannelBuilder, ItemBuilder};

pub fn generate_rss(posts: &[Post], config: &SiteConfig) -> String {
    let items: Vec<rss::Item> = posts
        .iter()
        .map(|post| {
            ItemBuilder::default()
                .title(Some(post.title.clone()))
                .link(Some(format!("{}/posts/{}", config.base_url, post.slug)))
                .description(Some(post.description.clone()))
                .content(Some(post.content_html.clone()))
                .pub_date(Some(post.date.to_string()))
                .build()
        })
        .collect();

    ChannelBuilder::default()
        .title(&config.title)
        .link(&config.base_url)
        .description(&config.description)
        .items(items)
        .build()
        .to_string()
}
