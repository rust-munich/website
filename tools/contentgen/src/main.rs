use anyhow::{Context, Result};
use chrono::{Local, NaiveDate};
use clap::{Parser, Subcommand};
use serde::{Deserialize, Serialize};
use std::{
    fs,
    path::{Path, PathBuf},
};

#[derive(Debug, Parser)]
#[command(
    name = "contentgen",
    version,
    about = "Generate Zola content pages from YAML data (events and jobs)"
)]
struct Cli {
    /// Path to the site root (where config.toml exists)
    #[arg(short, long, default_value = "../..")]
    root: PathBuf,

    #[command(subcommand)]
    cmd: Commands,
}

#[derive(Debug, Subcommand)]
enum Commands {
    /// Generate Zola event pages from /data/events/*.yml
    Events,
    /// Generate Zola job pages from /data/jobs/*.yml
    Jobs,
    /// Publish a single event to Meetup using GraphQL.
    /// Requires MEETUP_ACCESS_TOKEN in env and groupId or groupUrlname.
    PublishEvent {
        /// Event YAML id (filename without extension) in data/events/
        id: String,
        /// Meetup group id (e.g. "12345678"). Optional if group_urlname is provided.
        #[arg(long)]
        group_id: Option<String>,
        /// Meetup group urlname (e.g. "rust-munich"). Optional if group_id is provided.
        #[arg(long)]
        group_urlname: Option<String>,
        /// Actually publish (default false creates a draft). Use --publish to publish.
        #[arg(long)]
        publish: bool,
    },
}

#[derive(Debug, Serialize, Deserialize)]
struct Event {
    id: String,
    title: String,
    date: String, // YYYY-MM-DD
    time: Option<String>,
    venue: Option<String>,
    address: Option<String>,
    city: Option<String>,
    tags: Option<Vec<String>>,
    meetup_url: Option<String>,
    slides_url: Option<String>,
    youtube_url: Option<String>,
    ical_url: Option<String>,
    speakers: Option<Vec<String>>,
    language: Option<String>,
    draft: Option<bool>,
    lat: Option<f64>,
    lon: Option<f64>,
    description: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct Job {
    id: String,
    title: String,
    company: String,
    location: String,
    #[serde(rename = "type")]
    job_type: String,
    remote: String,
    experience: String,
    posted_date: String,          // YYYY-MM-DD
    expires_date: Option<String>, // YYYY-MM-DD
    salary_range: Option<String>,
    company_url: Option<String>,
    application_url: String,
    logo_url: Option<String>,
    tags: Option<Vec<String>>,
    draft: Option<bool>,
    description: String,
    requirements: Option<Vec<String>>,
    benefits: Option<Vec<String>>,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.cmd {
        Commands::Events => generate_events(&cli.root),
        Commands::Jobs => generate_jobs(&cli.root),
        Commands::PublishEvent {
            id,
            group_id,
            group_urlname,
            publish,
        } => publish_event(&cli.root, &id, group_id, group_urlname, publish).await,
    }
}

fn generate_events(root: &Path) -> Result<()> {
    let data_dir = root.join("data").join("events");
    let upcoming_dir = root.join("content").join("upcoming");
    let past_dir = root.join("content").join("past");
    fs::create_dir_all(&upcoming_dir)?;
    fs::create_dir_all(&past_dir)?;

    let today = Local::now().date_naive();

    for entry in
        fs::read_dir(&data_dir).with_context(|| format!("reading {}", data_dir.display()))?
    {
        let entry = entry?;
        if entry.file_type()?.is_file()
            && entry
                .path()
                .extension()
                .map(|e| e == "yml" || e == "yaml")
                .unwrap_or(false)
        {
            let s = fs::read_to_string(entry.path())?;
            let event: Event = serde_yaml::from_str(&s)
                .with_context(|| format!("parsing {}", entry.path().display()))?;
            let date = NaiveDate::parse_from_str(&event.date, "%Y-%m-%d").with_context(|| {
                format!("invalid date {} in {}", event.date, entry.path().display())
            })?;

            let is_past = date < today;
            let filename = format!("{}-{}.md", event.date, slugify(&event.title));

            let extra = build_event_extra(&event, is_past);
            let content = format!(
                r#"+++
title = "{title}"
template = "event.html"
[extra]
{extra}
+++

{body}
"#,
                title = escape_toml(&event.title),
                extra = extra,
                body = event.description.as_deref().unwrap_or("").trim_end()
            );

            let target = if is_past {
                past_dir.join(filename)
            } else {
                upcoming_dir.join(filename)
            };
            fs::write(target, content)?;
        }
    }
    println!("Generated event pages.");
    Ok(())
}

fn generate_jobs(root: &Path) -> Result<()> {
    let data_dir = root.join("data").join("jobs");
    let jobs_dir = root.join("content").join("jobs");

    // Ensure the jobs content directory exists
    fs::create_dir_all(&jobs_dir)?;

    let today = Local::now().date_naive();

    for entry in
        fs::read_dir(&data_dir).with_context(|| format!("reading {}", data_dir.display()))?
    {
        let entry = entry?;
        if entry.file_type()?.is_file()
            && entry
                .path()
                .extension()
                .map(|e| e == "yml" || e == "yaml")
                .unwrap_or(false)
        {
            let s = fs::read_to_string(entry.path())?;
            let job: Job = serde_yaml::from_str(&s)
                .with_context(|| format!("parsing {}", entry.path().display()))?;

            // Skip draft jobs
            if job.draft.unwrap_or(false) {
                continue;
            }

            // Check if job is expired
            if let Some(expires) = &job.expires_date {
                let expires_date =
                    NaiveDate::parse_from_str(expires, "%Y-%m-%d").with_context(|| {
                        format!(
                            "invalid expires_date {} in {}",
                            expires,
                            entry.path().display()
                        )
                    })?;
                if expires_date < today {
                    continue; // Skip expired jobs
                }
            }

            let filename = format!("{}.md", job.id);
            let extra = build_job_extra(&job);
            let content = format!(
                r#"+++
title = "{title}"
template = "job.html"
[extra]
{extra}
+++

{body}
"#,
                title = escape_toml(&job.title),
                extra = extra,
                body = format_job_content(&job)
            );

            let target = jobs_dir.join(filename);
            fs::write(target, content)?;
        }
    }
    println!("Generated job pages.");
    Ok(())
}

// Event helper functions
fn slugify(s: &str) -> String {
    let mut slug = s.to_lowercase();
    slug = slug
        .replace(['ä', 'Ä'], "ae")
        .replace(['ö', 'Ö'], "oe")
        .replace(['ü', 'Ü'], "ue")
        .replace('ß', "ss");
    slug.chars()
        .map(|c| if c.is_alphanumeric() { c } else { '-' })
        .collect::<String>()
        .split('-')
        .filter(|p| !p.is_empty())
        .collect::<Vec<_>>()
        .join("-")
}

fn build_event_extra(e: &Event, is_past: bool) -> String {
    // Build TOML under [extra]
    let mut parts = vec![format!(r#"date = "{}""#, e.date)];
    if let Some(t) = &e.time {
        parts.push(format!(r#"time = "{}""#, t));
    }
    if let Some(v) = &e.venue {
        parts.push(format!(r#"venue = "{}""#, v.replace('"', "\\\"")));
    }
    if let Some(a) = &e.address {
        parts.push(format!(r#"address = "{}""#, a.replace('"', "\\\"")));
    }
    if let Some(c) = &e.city {
        parts.push(format!(r#"city = "{}""#, c));
    }
    if let Some(tags) = &e.tags {
        let tags_toml = tags
            .iter()
            .map(|t| format!(r#""{}""#, t))
            .collect::<Vec<_>>()
            .join(", ");
        parts.push(format!("tags = [{}]", tags_toml));
    }
    if let Some(u) = &e.meetup_url {
        parts.push(format!(r#"meetup_url = "{}""#, u));
    }
    if let Some(u) = &e.slides_url {
        parts.push(format!(r#"slides_url = "{}""#, u));
    }
    if let Some(u) = &e.youtube_url {
        parts.push(format!(r#"youtube_url = "{}""#, u));
    }
    if let Some(u) = &e.ical_url {
        parts.push(format!(r#"ical_url = "{}""#, u));
    }
    if let Some(s) = &e.speakers {
        let speakers_toml = s
            .iter()
            .map(|x| format!(r#""{}""#, x))
            .collect::<Vec<_>>()
            .join(", ");
        parts.push(format!("speakers = [{}]", speakers_toml));
    }
    if let (Some(lat), Some(lon)) = (e.lat, e.lon) {
        let embed = format!(
            r#"<iframe src="https://www.openstreetmap.org/export/embed.html?bbox={lon}%2C{lat}%2C{lon}%2C{lat}&layer=mapnik&marker={lat}%2C{lon}" width="100%" height="300" style="border:1px solid var(--border)"></iframe>"#
        );
        parts.push(format!(r#"map_embed = "{}""#, embed.replace('"', "\\\"")));
    }
    parts.push(format!("is_past = {}", is_past));
    parts.push(format!("is_upcoming = {}", !is_past));
    parts.join("\n")
}

// Job helper functions
fn build_job_extra(job: &Job) -> String {
    let mut parts = vec![
        format!(r#"company = "{}""#, escape_toml(&job.company)),
        format!(r#"location = "{}""#, escape_toml(&job.location)),
        format!(r#"job_type = "{}""#, escape_toml(&job.job_type)),
        format!(r#"remote = "{}""#, escape_toml(&job.remote)),
        format!(r#"experience = "{}""#, escape_toml(&job.experience)),
        format!(r#"posted_date = "{}""#, job.posted_date),
        format!(r#"application_url = "{}""#, job.application_url),
    ];

    if let Some(expires) = &job.expires_date {
        parts.push(format!(r#"expires_date = "{}""#, expires));
    }
    if let Some(salary) = &job.salary_range {
        parts.push(format!(r#"salary_range = "{}""#, escape_toml(salary)));
    }
    if let Some(url) = &job.company_url {
        parts.push(format!(r#"company_url = "{}""#, url));
    }
    if let Some(logo) = &job.logo_url {
        parts.push(format!(r#"logo_url = "{}""#, logo));
    }
    if let Some(tags) = &job.tags {
        let tags_toml = tags
            .iter()
            .map(|t| format!(r#""{}""#, escape_toml(t)))
            .collect::<Vec<_>>()
            .join(", ");
        parts.push(format!("tags = [{}]", tags_toml));
    }

    parts.join("\n")
}

fn format_job_content(job: &Job) -> String {
    let mut content = String::new();

    // Main description
    content.push_str(&job.description);
    content.push_str("\n\n");

    // Requirements section
    if let Some(requirements) = &job.requirements {
        content.push_str("## Requirements\n\n");
        for req in requirements {
            content.push_str(&format!("- {}\n", req));
        }
        content.push_str("\n");
    }

    // Benefits section
    if let Some(benefits) = &job.benefits {
        content.push_str("## Benefits\n\n");
        for benefit in benefits {
            content.push_str(&format!("- {}\n", benefit));
        }
        content.push_str("\n");
    }

    content.trim_end().to_string()
}

// Shared helper functions
fn escape_toml(s: &str) -> String {
    s.replace('"', "\\\"")
}

// Meetup publishing functionality (from eventgen)
#[derive(Serialize)]
struct GraphQLRequest<'a> {
    query: &'a str,
    variables: serde_json::Value,
}

async fn meetup_graphql(token: &str, body: &GraphQLRequest<'_>) -> Result<serde_json::Value> {
    let client = reqwest::Client::new();
    let resp = client
        .post("https://api.meetup.com/gql")
        .bearer_auth(token)
        .json(body)
        .send()
        .await?;
    let status = resp.status();
    let text = resp.text().await?;
    if !status.is_success() {
        anyhow::bail!("Meetup API error {}: {}", status, text);
    }
    let v: serde_json::Value = serde_json::from_str(&text)?;
    Ok(v)
}

async fn publish_event(
    root: &Path,
    id: &str,
    group_id: Option<String>,
    group_urlname: Option<String>,
    publish: bool,
) -> Result<()> {
    let path = root.join("data").join("events").join(format!("{id}.yml"));
    let s = fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))?;
    let e: Event = serde_yaml::from_str(&s)?;

    let token = std::env::var("MEETUP_ACCESS_TOKEN")
        .context("MEETUP_ACCESS_TOKEN missing in environment")?;

    // Build Meetup CreateEventInput based on available fields
    // NOTE: Meetup uses GraphQL; see https://www.meetup.com/api/guide/
    let input = build_create_event_input(&e, group_id, group_urlname, publish)?;

    let gql = r#"
mutation($input: CreateEventInput!) {
  createEvent(input: $input) {
    event { id title status timeZone startAt endAt eventUrl }
    errors { message code field }
  }
}"#;

    let req = GraphQLRequest {
        query: gql,
        variables: serde_json::json!({ "input": input }),
    };

    let resp = meetup_graphql(&token, &req).await?;
    println!("{}", serde_json::to_string_pretty(&resp)?);
    if let Some(errors) = resp
        .get("data")
        .and_then(|d| d.get("createEvent"))
        .and_then(|c| c.get("errors"))
    {
        if !errors.as_array().unwrap_or(&vec![]).is_empty() {
            anyhow::bail!("Meetup returned errors: {}", errors);
        }
    }
    Ok(())
}

fn build_create_event_input(
    e: &Event,
    group_id: Option<String>,
    group_urlname: Option<String>,
    publish: bool,
) -> Result<serde_json::Value> {
    // Minimal viable fields: groupId or groupUrlname, title, description, startAt (ISO8601), venue (if any)
    // Assume Europe/Berlin timezone.
    let tz = "Europe/Berlin";
    let date = NaiveDate::parse_from_str(&e.date, "%Y-%m-%d")?;
    let time = e.time.as_deref().unwrap_or("19:00");
    let dt = chrono::NaiveTime::parse_from_str(time, "%H:%M")?;
    let start = chrono::NaiveDateTime::new(date, dt);
    // default 3 hours duration
    let end = start + chrono::Duration::hours(3);

    let start_iso = chrono::DateTime::<chrono::FixedOffset>::from_local(
        start,
        // CET/CEST offset is tricky; we let Meetup use tz field and just send naive? But API wants ISO.
        // Use +01:00 as baseline; it's fine with tz below.
        chrono::FixedOffset::east_opt(1 * 3600).unwrap(),
    )
    .to_rfc3339();

    let end_iso = chrono::DateTime::<chrono::FixedOffset>::from_local(
        end,
        chrono::FixedOffset::east_opt(1 * 3600).unwrap(),
    )
    .to_rfc3339();

    let mut input = serde_json::json!({
        "title": e.title,
        "description": e.description.clone().unwrap_or_default(),
        "startAt": start_iso,
        "endAt": end_iso,
        "timeZone": tz,
        "publishStatus": if publish { "PUBLISHED" } else { "DRAFT" },
    });

    if let Some(id) = group_id {
        input["groupId"] = serde_json::Value::String(id);
    }
    if let Some(urlname) = group_urlname {
        input["groupUrlname"] = serde_json::Value::String(urlname);
    }

    if let Some(venue) = &e.venue {
        // Meetup might require a separate venue object; here we just include the name in title/description.
        input["title"] = serde_json::Value::String(format!("{} @ {}", e.title, venue));
    }

    Ok(input)
}
