use crate::config::SiteConfig;
use crate::content::Post;

pub fn generate_sitemap(posts: &[Post], tags: &[String], config: &SiteConfig) -> String {
    let mut urls = vec![format!(
        "  <url>\n    <loc>{}</loc>\n    <changefreq>weekly</changefreq>\n    <priority>1.0</priority>\n  </url>",
        config.base_url
    )];

    for post in posts {
        urls.push(format!(
            "  <url>\n    <loc>{}/posts/{}</loc>\n    <lastmod>{}</lastmod>\n    <changefreq>monthly</changefreq>\n    <priority>0.8</priority>\n  </url>",
            config.base_url, post.slug, post.date
        ));
    }

    for tag in tags {
        urls.push(format!(
            "  <url>\n    <loc>{}/tags/{}</loc>\n    <changefreq>weekly</changefreq>\n    <priority>0.5</priority>\n  </url>",
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
