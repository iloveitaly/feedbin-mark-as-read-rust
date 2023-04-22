use chrono::prelude::*;
use futures::StreamExt;
use reqwest::header::HeaderValue;
use serde_json::Value;
use std::env;
use std::process;
use structopt::StructOpt;
use tokio;

const FEEDBIN_BASE_URL: &str = "https://api.feedbin.com/";

#[derive(StructOpt)]
struct Cli {
    /// Perform a dry run without marking entries as read
    #[structopt(long)]
    dry_run: bool,
}

async fn one_month_old(entry_id: u64, feedbin_auth: &HeaderValue) -> Option<String> {
    let client = reqwest::Client::new();
    let url = format!("{}/v2/entries/{}.json", FEEDBIN_BASE_URL, entry_id);

    let response = client
        .get(&url)
        .header("Authorization", feedbin_auth.clone())
        .send()
        .await
        .unwrap();

    let entry: Value = response.json().await.unwrap();

    let published = &entry["published"].as_str().unwrap();
    let published_date = DateTime::parse_from_rfc3339(published).unwrap();
    let one_month_ago = Utc::now() - chrono::Duration::days(30);

    if published_date < one_month_ago {
        Some(entry["url"].as_str().unwrap().to_string())
    } else {
        None
    }
}

async fn mark_as_read(entry_ids: Vec<u64>, feedbin_auth: &HeaderValue) {
    let client = reqwest::Client::new();
    let url = format!("{}/v2/unread_entries.json", FEEDBIN_BASE_URL);
    let entry_ids: Vec<String> = entry_ids.iter().map(|id| id.to_string()).collect();
    let entry_ids = entry_ids.join(",");

    client
        .delete(&url)
        .header("Authorization", feedbin_auth.clone())
        .header("Content-Type", "application/json")
        .body(format!("{{\"unread_entries\": [{}]}}", entry_ids))
        .send()
        .await
        .unwrap();
}

#[tokio::main]
async fn main() {
    let args = Cli::from_args();

    let feedbin_auth = env::var("FEEDBIN_AUTH").unwrap_or_else(|_| {
        eprintln!("Error: FEEDBIN_AUTH environment variable not set");
        process::exit(1);
    });

    let feedbin_auth = format!("Basic {}", base64::encode(&feedbin_auth));
    let feedbin_auth = HeaderValue::from_str(&feedbin_auth).unwrap();

    println!("getting all unread entries");

    let client = reqwest::Client::new();
    let url = format!("{}/v2/unread_entries.json", FEEDBIN_BASE_URL);
    let response = client
        .get(&url)
        .header("Authorization", feedbin_auth.clone())
        .send()
        .await
        .unwrap();

    let unread_entries: Vec<u64> = response.json().await.unwrap();

    let one_month_old_urls: Vec<String> =
        futures::stream::iter(unread_entries.into_iter().take(50))
            .filter_map(|entry_id| {
                let feedbin_auth = feedbin_auth.clone();
                async move { one_month_old(entry_id, &feedbin_auth).await }
            })
            .collect()
            .await;

    println!("{} entries are one month old", one_month_old_urls.len());

    let entry_ids_to_mark_as_read: Vec<u64> = one_month_old_urls
        .iter()
        .map(|url| {
            println!("{}", url);
            url.parse::<u64>().unwrap()
        })
        .collect();

    if !args.dry_run {
        mark_as_read(entry_ids_to_mark_as_read, &feedbin_auth).await;
    } else {
        // include number of entries in log line
        println!("dry run, not marking {} entries as read", entry_ids_to_mark_as_read.len());
    }

    println!("feeds updated");
}
