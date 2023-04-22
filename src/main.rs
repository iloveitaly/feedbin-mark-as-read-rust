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

// example entry:
// {
//     "author": "Blake Oliver",
//     "content": "<p>Gary Bolinger writes in Accounting Today that <a href=\"https://www.accountingtoday.com/opinion/the-150-hour-rule-is-not-the-problem\">\"the 150-hour rule is not the problem\"</a> and recommends that a solution to the talent shortage is \"partners need to share the wealth\" by increasing starting salaries.</p><p>So let me get this straight.</p><p>The partners — who worked really hard for an average of 15 years to get there — should bear the cost of the labor shortage. We'd rather ask them to do that than consider an obvious way to reduce the cost to become a CPA, and thereby increase the supply of talent.</p><p>I agree that the cost of the 150-hour rule would not be as much of a problem if starting salaries increased — but telling partners to \"share the wealth\" as a solution is insulting. It also ignores the plight of students, who continue to go deeper and deeper into debt while rewarding colleges and universities that have failed to modernize their curricula to prepare students for work in the modern world.</p><p>Yes, replacing the extra 30 semester hours of education with a year of work experience could hurt master of accountancy enrollments. But it would help firms. And I think colleges and universities can handle it. The best programs will continue to attract students who see the value.</p><p>I also agree with Gary's argument that the 150-hour rule is not the biggest problem. I don't believe anyone is making that argument. But it is a contributing factor and a problem that is relatively easy to solve in the short term — if our leaders had the will to do it.</p>",
//     "created_at": "2023-03-22T21:07:11.266265Z",
//     "extracted_content_url": "https://extract.feedbin.com/parser/feedbin/8d16b8970ad155745f867e98b8769f7c8cc8232d?base64_url=aHR0cHM6Ly93d3cuYmxha2VvbGl2ZXIuY29tL2Jsb2cvc2hhcmluZy10aGUtd2VhbHRoLWlzLWFza2luZy1wYXJ0bmVycy10by1pbmNyZWFzZS1zYWxhcmllcy10aGUtYW5zd2VyLXRvLXRoZS1hY2NvdW50aW5nLXRhbGVudC1zaG9ydGFnZQ==",
//     "feed_id": 2104763,
//     "id": 4048174490,
//     "published": "2023-03-22T19:30:07.000000Z",
//     "summary": "Gary Bolinger writes in Accounting Today that \"the 150-hour rule is not the problem\" and recommends that a solution to the talent shortage is \"partners need to share the wealth\" by increasing starting salaries. So let me get this straight. The partners —",
//     "title": "Sharing the Wealth: Is Asking Partners to Increase Salaries the Answer to the Accounting Talent Shortage?",
//     "url": "https://www.blakeoliver.com/blog/sharing-the-wealth-is-asking-partners-to-increase-salaries-the-answer-to-the-accounting-talent-shortage"
// }

async fn one_month_old(entry_id: u64, feedbin_auth: &HeaderValue) -> Option<(String, u64)> {
    let client = reqwest::Client::new();
    let url = format!("{}/v2/entries/{}.json", FEEDBIN_BASE_URL, entry_id);

    let response = client
        .get(&url)
        .header("Authorization", feedbin_auth.clone())
        .send()
        .await
        .unwrap();

    let entry: Value = response.json().await.unwrap();

    // TODO what is this weird syntax?
    println!("{:#?}", entry);

    let published = &entry["published"].as_str().unwrap();
    let published_date = DateTime::parse_from_rfc3339(published).unwrap();
    let one_month_ago = Utc::now() - chrono::Duration::days(30);

    if published_date < one_month_ago {
        Some((entry["url"].as_str().unwrap().to_string(), entry["id"].as_u64().unwrap()))
    } else {
        None
    }
}

async fn mark_as_read(entry_ids: Vec<u64>, feedbin_auth: &HeaderValue) {
    let client = reqwest::Client::new();
    let url = format!("{}/v2/unread_entries.json", FEEDBIN_BASE_URL);
    let entry_ids: Vec<String> = entry_ids.iter().map(|id| id.to_string()).collect();
    let entry_ids = entry_ids.join(",");

    println!("marking as read: {}", entry_ids);

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

    let one_month_old_urls: Vec<(String, u64)> =
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
        .map(|entry| {
            println!("entry url {}", entry.0);
            entry.1
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
