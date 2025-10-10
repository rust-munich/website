# Rust Munich — Zola site

This repository contains the Zola static site for **rust-munich.de** and a small Rust CLI to generate event pages from YAML and (optionally) publish them to Meetup via GraphQL.

## Prereqs

- [Zola](https://www.getzola.org/documentation/getting-started/installation/)
- Rust toolchain (for the CLI)

## Structure

```
.
├── Cargo.toml              # workspace (CLI)
├── config.toml             # Zola config (en default, de alternative)
├── content/                # Pages; events are generated into upcoming/ and past/
├── data/events/            # Single source of truth: one YAML per event
├── static/img/             # Logo placeholder (replace with icon repo asset)
└── tools/contentgen/       # Rust CLI for events and jobs generation
```

## Local development

to keep things simple, there is a Makefile hiding the details..

```bash
# one time action:
make setup
# use the content generators
make build
# watches for changes and serves locally
make serve
```

## Adding an event

Create a new file in `data/events/rust-munich-YYYY-number.yml`:

```yaml
id: "rust-munich-2025-2"
title: "Rust Munich 2025/2"
date: "2025-11-12"
time: "19:00"
venue: "YourVenue"
address: "Street 1"
city: "Munich"
tags: ["talks"]
meetup_url: "https://www.meetup.com/de-DE/rust-munich/"
slides_url: "https://github.com/rust-munich/slides"
youtube_url: ""
ical_url: ""
speakers: ["You?"]
language: "en"
draft: false
description: |
  Talk abstract here.
```

Then regenerate:

```bash
cargo eventgen
```

## Publishing to Meetup (optional)

TODO: this section needs overhaul!! the cli arguments are not correct anymore

The CLI can publish as a draft or publish immediately using Meetup's GraphQL API (OAuth2). You need an **access token** in `MEETUP_ACCESS_TOKEN` and either your group's **id** or **urlname**.

> See Meetup docs: https://www.meetup.com/api/guide/ and https://www.meetup.com/api/authentication/

```bash
# Draft (safer)
MEETUP_ACCESS_TOKEN=... cargo run -p eventgen -- --root . publish 2025-11-12-rust-munich-2 --group-urlname rust-munich

# Publish
MEETUP_ACCESS_TOKEN=... cargo run -p eventgen -- --root . publish 2025-11-12-rust-munich-2 --group-urlname rust-munich --publish
```

**Note:** The CLI submits a minimal `CreateEventInput`. You may extend `build_create_event_input()` to include RSVP caps, visibility, and a proper venue object once you have venue IDs configured.

## Theming

Typography uses **Fira Sans** (UI/headings) and **Source Serif 4** (body), with Rust-like accents (`#dea584`). Colors and contrasts are tuned to be close to the Rust blog while keeping WCAG AA.

Replace the placeholder logo at `static/img/rust-munich-logo.svg` with the SVG from https://github.com/rust-munich/icon .

## Translations (i18n)

Default language is English (`en`), with German (`de`) available for localized pages if desired. Add translated markdown files under `content/<section>/_index.<lang>.md` etc.

## Deployment

- **Netlify**: set the build command to `zola build` and publish directory to `public/`.
- **GitHub Pages**: build via GitHub Actions, upload `public/` to `gh-pages`.
- **Vercel**: use a Build Step invoking `zola build`.

## License

- Site content: CC BY 4.0
- Code: MIT OR Apache-2.0
