use chrono::Local;
use clap::{Parser, Subcommand};
use forge::config::SiteConfig;
use forge::content::load_posts;
use forge::renderer::Renderer;
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use walkdir::WalkDir;

#[derive(Parser)]
#[command(name = "forge", version, about = "a rust static site generator")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Build,
    New { title: String },
    Clean,
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Build => build(),
        Commands::New { title } => new_post(&title),
        Commands::Clean => {
            let output = Path::new("output");
            if output.exists() {
                fs::remove_dir_all(output).expect("failed to clean output");
            }
            println!("cleaned output directory");
        }
    }
}

fn new_post(title: &str) {
    let slug = title.to_lowercase().replace(' ', "-");
    let date = Local::now().format("%Y-%m-%d");
    let path = Path::new("content/posts").join(format!("{}.md", slug));

    if path.exists() {
        eprintln!("post already exists: {}", path.display());
        std::process::exit(1);
    }

    fs::create_dir_all("content/posts").expect("failed to create content dir");
    fs::write(
        &path,
        format!(
            "---\ntitle: \"{}\"\ndescription: \"\"\ndate: {}\ntags: []\ndraft: true\n---\n",
            title, date
        ),
    )
    .expect("failed to write post");

    println!("created {}", path.display());
}

fn build() {
    let start = std::time::Instant::now();
    let config = SiteConfig::load(Path::new("forge.toml"));
    let posts = load_posts(Path::new("content"));
    let renderer = Renderer::new(Path::new("templates"));

    let output = Path::new("output");
    if output.exists() {
        fs::remove_dir_all(output).expect("failed to clean output");
    }
    fs::create_dir_all(output.join("posts")).expect("failed to create output dirs");
    fs::create_dir_all(output.join("tags")).expect("failed to create output dirs");

    // render index
    let index_html = renderer.render_index(&posts, &config);
    fs::write(output.join("index.html"), index_html).expect("failed to write index");

    // render 404
    let not_found_html = renderer.render_404(&config);
    fs::write(output.join("404.html"), not_found_html).expect("failed to write 404");

    // render posts
    for (i, post) in posts.iter().enumerate() {
        let prev = if i + 1 < posts.len() { Some(&posts[i + 1]) } else { None };
        let next = if i > 0 { Some(&posts[i - 1]) } else { None };
        let html = renderer.render_post(post, prev, next, &config);
        let post_dir = output.join("posts").join(&post.slug);
        fs::create_dir_all(&post_dir).expect("failed to create post dir");
        fs::write(post_dir.join("index.html"), html).expect("failed to write post");
    }

    // collect and render tag pages
    let mut tags: HashMap<String, Vec<&forge::content::Post>> = HashMap::new();
    for post in &posts {
        for tag in &post.tags {
            tags.entry(tag.clone()).or_default().push(post);
        }
    }

    for (tag, tag_posts) in &tags {
        let html = renderer.render_tag(tag, tag_posts, &config);
        let tag_dir = output.join("tags").join(tag);
        fs::create_dir_all(&tag_dir).expect("failed to create tag dir");
        fs::write(tag_dir.join("index.html"), html).expect("failed to write tag page");
    }

    // render tags index
    let mut tag_counts: Vec<(String, usize)> = tags
        .iter()
        .map(|(tag, posts)| (tag.clone(), posts.len()))
        .collect();
    tag_counts.sort_by(|a, b| b.1.cmp(&a.1));
    let tags_index_html = renderer.render_tags_index(&tag_counts, &config);
    fs::write(output.join("tags").join("index.html"), tags_index_html)
        .expect("failed to write tags index");

    // generate rss feed
    let rss = forge::feed::generate_rss(&posts, &config);
    fs::write(output.join("feed.xml"), rss).expect("failed to write rss feed");

    // generate sitemap and robots.txt
    let all_tags: Vec<String> = tags.keys().cloned().collect();
    let sitemap = forge::seo::generate_sitemap(&posts, &all_tags, &config);
    fs::write(output.join("sitemap.xml"), sitemap).expect("failed to write sitemap");

    let robots = forge::seo::generate_robots(&config);
    fs::write(output.join("robots.txt"), robots).expect("failed to write robots.txt");

    // copy static assets
    let static_dir = Path::new("static");
    if static_dir.exists() {
        copy_dir(static_dir, &output.join("static"));
    }

    // copy cloudflare pages files to output root
    for file in &["_headers"] {
        let src = Path::new(file);
        if src.exists() {
            fs::copy(src, output.join(file)).expect("failed to copy cloudflare config");
        }
    }

    let elapsed = start.elapsed();
    println!(
        "built {} posts, {} tags in {:.0?}",
        posts.len(),
        tags.len(),
        elapsed
    );
}

fn copy_dir(src: &Path, dst: &Path) {
    for entry in WalkDir::new(src).into_iter().filter_map(|e| e.ok()) {
        let target = dst.join(entry.path().strip_prefix(src).unwrap());
        if entry.file_type().is_dir() {
            fs::create_dir_all(&target).expect("failed to create static dir");
        } else {
            fs::copy(entry.path(), &target).expect("failed to copy static file");
        }
    }
}
