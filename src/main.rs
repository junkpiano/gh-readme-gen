mod github;
mod template;

use anyhow::Result;
use chrono::NaiveDate;
use clap::Parser;
use github::GitHubClient;
use std::collections::{HashMap, HashSet};
use std::fs;

#[derive(Parser)]
#[command(name = "gh-readme-gen", about = "Generate a GitHub profile README")]
struct Cli {
    /// GitHub username
    username: String,

    /// GitHub personal access token (optional, increases rate limit)
    #[arg(short, long, env = "GITHUB_TOKEN")]
    token: Option<String>,

    /// Output file (defaults to stdout)
    #[arg(short, long)]
    output: Option<String>,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    let cutoff_days: i64 = if cli.token.is_some() { 180 } else { 90 };
    let client = GitHubClient::new(cli.token)?;

    eprintln!("Fetching profile for {}...", cli.username);
    let (user, repos, events) = tokio::try_join!(
        client.get_user(&cli.username),
        client.get_repos(&cli.username),
        client.get_events(&cli.username, cutoff_days),
    )?;
    eprintln!("Fetched {} repos, {} events.", repos.len(), events.len());

    // Build daily contribution map.
    // With a token: use GraphQL contribution calendar (accurate, full history).
    // Without: count push events per day from the Events API (best effort).
    let daily: HashMap<NaiveDate, u32> = if client.has_token() {
        eprintln!("Fetching contribution calendar...");
        client
            .get_contribution_calendar(&cli.username, cutoff_days)
            .await
            .unwrap_or_else(|e| {
                eprintln!("Warning: GraphQL failed ({e}), falling back to events.");
                events_to_daily(&events)
            })
    } else {
        events_to_daily(&events)
    };

    // Fetch full PR titles (Events API only gives a truncated PR object)
    let pr_pairs: HashSet<(String, u64)> = events.iter()
        .filter(|e| e.kind == "PullRequestEvent")
        .filter_map(|e| {
            let number = e.payload.get("number").and_then(|v| v.as_u64())?;
            Some((e.repo.name.clone(), number))
        })
        .collect();

    let mut pr_titles: HashMap<String, String> = HashMap::new();
    for (repo, number) in &pr_pairs {
        if let Ok(title) = client.get_pr_title(repo, *number).await {
            pr_titles.insert(format!("{repo}#{number}"), title);
        }
    }

    let readme = template::render(&user, &repos, &events, &daily, cutoff_days, &pr_titles);

    match cli.output {
        Some(path) => {
            fs::write(&path, &readme)?;
            eprintln!("Written to {path}");
        }
        None => print!("{readme}"),
    }

    Ok(())
}

fn events_to_daily(events: &[github::Event]) -> HashMap<NaiveDate, u32> {
    let mut daily: HashMap<NaiveDate, u32> = HashMap::new();
    for e in events {
        if e.kind == "PushEvent" {
            *daily.entry(e.created_at.date_naive()).or_default() += 1;
        }
    }
    daily
}
