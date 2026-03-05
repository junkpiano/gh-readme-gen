use anyhow::Result;
use chrono::{DateTime, Duration, NaiveDate, Utc};
use reqwest::Client;
use serde::Deserialize;
use std::collections::HashMap;

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

    pub fn has_token(&self) -> bool {
        self.token.is_some()
    }

    /// Fetches the contribution calendar via GraphQL (requires token).
    /// Returns a map of date → total contribution count.
    pub async fn get_contribution_calendar(
        &self,
        username: &str,
        days: i64,
    ) -> Result<HashMap<NaiveDate, u32>> {
        let token = self
            .token
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("token required for GraphQL API"))?;

        let from = (Utc::now() - Duration::days(days))
            .format("%Y-%m-%dT%H:%M:%SZ")
            .to_string();
        let to = Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string();

        let query = format!(
            r#"query {{
  user(login: "{username}") {{
    contributionsCollection(from: "{from}", to: "{to}") {{
      contributionCalendar {{
        weeks {{
          contributionDays {{
            date
            contributionCount
          }}
        }}
      }}
    }}
  }}
}}"#
        );

        let resp: serde_json::Value = self
            .client
            .post("https://api.github.com/graphql")
            .header("Authorization", format!("Bearer {token}"))
            .json(&serde_json::json!({ "query": query }))
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;

        let mut daily: HashMap<NaiveDate, u32> = HashMap::new();
        if let Some(weeks) = resp
            .pointer("/data/user/contributionsCollection/contributionCalendar/weeks")
            .and_then(|v| v.as_array())
        {
            for week in weeks {
                if let Some(contrib_days) = week["contributionDays"].as_array() {
                    for day in contrib_days {
                        let date_str = day["date"].as_str().unwrap_or("");
                        let count = day["contributionCount"].as_u64().unwrap_or(0) as u32;
                        if let Ok(date) = NaiveDate::parse_from_str(date_str, "%Y-%m-%d") {
                            daily.insert(date, count);
                        }
                    }
                }
            }
        }
        Ok(daily)
    }

    /// Fetches public events page by page (max 10), stopping once events are
    /// older than `cutoff_days`.
    pub async fn get_events(&self, username: &str, cutoff_days: i64) -> Result<Vec<Event>> {
        let cutoff = Utc::now() - Duration::days(cutoff_days);
        let mut all = Vec::new();
        for page in 1..=10u8 {
            let url = format!(
                "{API_BASE}/users/{username}/events/public?per_page=30&page={page}"
            );
            let mut req = self.client.get(&url);
            if let Some(auth) = self.auth_header() {
                req = req.header("Authorization", auth);
            }
            let batch: Vec<Event> = req.send().await?.error_for_status()?.json().await?;
            if batch.is_empty() {
                break;
            }
            let oldest = batch.last().map(|e| e.created_at);
            all.extend(batch);
            if oldest.map(|t| t < cutoff).unwrap_or(false) {
                break;
            }
        }
        Ok(all)
    }
}
