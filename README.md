# forge

a rust static site generator for developer blogs.

## stack

- **generator** — rust cli (pulldown-cmark, tera, syntect, ammonia)
- **frontend** — vite 6, vanilla typescript, tailwind css v4
- **hosting** — cloudflare pages

## usage

```bash
# install dependencies
npm install

# create a new post
cargo run -- new "my post title"

# build the site
npm run build && cargo run -- build

# clean output
cargo run -- clean
```

## project structure

```
forge/
├── src/              # rust generator
├── frontend/         # vite + typescript + css
├── templates/        # tera html templates
├── content/posts/    # markdown posts
├── static/           # built css/js + assets
├── forge.toml        # site config
└── output/           # generated site
```

## content

posts live in `content/posts/` as markdown with yaml frontmatter:

```markdown
---
title: "my post"
description: "a short description"
date: 2026-03-08
tags: ["rust", "web"]
draft: false
---

post content here.
```

## deploy

connect the repo to cloudflare pages:

- **build command:** `npm install && npm run build && cargo run -- build`
- **output directory:** `output`
