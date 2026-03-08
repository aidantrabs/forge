use crate::config::SiteConfig;
use crate::content::Post;

pub fn generate_sitemap(posts: &[Post], tags: &[String], config: &SiteConfig) -> String {
    let mut urls = vec![format!("  <url><loc>{}</loc></url>", config.base_url)];

    for post in posts {
        urls.push(format!(
            "  <url><loc>{}/posts/{}</loc></url>",
            config.base_url, post.slug
        ));
    }

    for tag in tags {
        urls.push(format!(
            "  <url><loc>{}/tags/{}</loc></url>",
            config.base_url, tag
        ));
    }

    format!(
        "<?xml version=\"1.0\" encoding=\"utf-8\"?>\n\
         <urlset xmlns=\"http://www.sitemaps.org/schemas/sitemap/0.9\">\n\
         {}\n\
         </urlset>",
        urls.join("\n")
    )
}

pub fn generate_robots(config: &SiteConfig) -> String {
    format!(
        "User-agent: *\nAllow: /\n\nSitemap: {}/sitemap.xml",
        config.base_url
    )
}
