use clap::{Parser, Subcommand};
use forge::config::SiteConfig;
use forge::content::load_posts;
use forge::renderer::Renderer;
use std::collections::HashMap;
use std::fs;
use std::path::Path;

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
        Commands::New { title } => {
            println!("creating post: {}", title);
        }
        Commands::Clean => {
            let output = Path::new("output");
            if output.exists() {
                fs::remove_dir_all(output).expect("failed to clean output");
            }
            println!("cleaned output directory");
        }
    }
}

fn build() {
    let config = SiteConfig::load(Path::new("forge.toml"));
    let posts = load_posts(Path::new("content"));
    let renderer = Renderer::new(Path::new("templates"));

    let output = Path::new("output");
    fs::create_dir_all(output.join("posts")).expect("failed to create output dirs");
    fs::create_dir_all(output.join("tags")).expect("failed to create output dirs");

    // render index
    let index_html = renderer.render_index(&posts, &config);
    fs::write(output.join("index.html"), index_html).expect("failed to write index");

    // render posts
    for post in &posts {
        let html = renderer.render_post(post, &config);
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

    // generate rss feed
    let rss = forge::feed::generate_rss(&posts, &config);
    fs::write(output.join("feed.xml"), rss).expect("failed to write rss feed");

    println!("built {} posts, {} tags", posts.len(), tags.len());
}
