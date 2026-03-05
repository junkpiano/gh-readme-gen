mod github;
mod template;

use anyhow::Result;
use clap::Parser;
use github::GitHubClient;
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
    let client = GitHubClient::new(cli.token)?;

    eprintln!("Fetching profile for {}...", cli.username);
    let (user, repos, events) = tokio::try_join!(
        client.get_user(&cli.username),
        client.get_repos(&cli.username),
        client.get_events(&cli.username, 3),
    )?;
    eprintln!("Fetched {} repos, {} events.", repos.len(), events.len());

    let readme = template::render(&user, &repos, &events);

    match cli.output {
        Some(path) => {
            fs::write(&path, &readme)?;
            eprintln!("Written to {path}");
        }
        None => print!("{readme}"),
    }

    Ok(())
}
