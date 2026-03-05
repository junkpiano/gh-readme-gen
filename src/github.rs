use anyhow::Result;
use chrono::{DateTime, Utc};
use reqwest::Client;
use serde::Deserialize;

const API_BASE: &str = "https://api.github.com";

#[derive(Debug, Deserialize)]
pub struct User {
    pub login: String,
    pub name: Option<String>,
    pub bio: Option<String>,
    pub avatar_url: String,
    pub html_url: String,
    pub location: Option<String>,
    pub blog: Option<String>,
    pub twitter_username: Option<String>,
    pub public_repos: u32,
    pub followers: u32,
    pub following: u32,
}

#[derive(Debug, Deserialize)]
pub struct Repo {
    pub name: String,
    pub description: Option<String>,
    pub html_url: String,
    pub language: Option<String>,
    pub stargazers_count: u32,
    pub forks_count: u32,
    pub fork: bool,
}

#[derive(Debug, Deserialize)]
pub struct EventRepo {
    pub name: String,
}

#[derive(Debug, Deserialize)]
pub struct Event {
    #[serde(rename = "type")]
    pub kind: String,
    pub repo: EventRepo,
    pub created_at: DateTime<Utc>,
    pub payload: serde_json::Value,
}

pub struct GitHubClient {
    client: Client,
    token: Option<String>,
}

impl GitHubClient {
    pub fn new(token: Option<String>) -> Result<Self> {
        let client = Client::builder()
            .user_agent("gh-readme-gen/0.1.0")
            .build()?;
        Ok(Self { client, token })
    }

    fn auth_header(&self) -> Option<String> {
        self.token.as_ref().map(|t| format!("Bearer {t}"))
    }

    pub async fn get_user(&self, username: &str) -> Result<User> {
        let url = format!("{API_BASE}/users/{username}");
        let mut req = self.client.get(&url);
        if let Some(auth) = self.auth_header() {
            req = req.header("Authorization", auth);
        }
        let user = req.send().await?.error_for_status()?.json::<User>().await?;
        Ok(user)
    }

    pub async fn get_repos(&self, username: &str) -> Result<Vec<Repo>> {
        let url = format!("{API_BASE}/users/{username}/repos?per_page=100&sort=stars");
        let mut req = self.client.get(&url);
        if let Some(auth) = self.auth_header() {
            req = req.header("Authorization", auth);
        }
        let repos = req.send().await?.error_for_status()?.json::<Vec<Repo>>().await?;
        Ok(repos)
    }

    /// Fetches up to `pages` pages of public events (30 events/page, max 10 pages).
    pub async fn get_events(&self, username: &str, pages: u8) -> Result<Vec<Event>> {
        let mut all = Vec::new();
        for page in 1..=pages {
            let url = format!(
                "{API_BASE}/users/{username}/events/public?per_page=30&page={page}"
            );
            let mut req = self.client.get(&url);
            if let Some(auth) = self.auth_header() {
                req = req.header("Authorization", auth);
            }
            let batch: Vec<Event> = req.send().await?.error_for_status()?.json().await?;
            let done = batch.is_empty();
            all.extend(batch);
            if done {
                break;
            }
        }
        Ok(all)
    }
}
