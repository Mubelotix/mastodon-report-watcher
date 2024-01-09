use std::env;
use minreq::get;
use serde::Deserialize;
use time::OffsetDateTime;

#[derive(Debug, Deserialize)]
struct Report {
    action_taken: bool,
    #[serde(with = "time::serde::iso8601")]
    created_at: OffsetDateTime,
}

fn should_shutdown(token: &str) -> bool {
    let response = get("https://mastodon.insa.lol/api/v1/admin/reports")
        .with_header("Authorization", format!("Bearer {}", token))
        .send()
        .expect("Failed to send report request");

    if response.status_code != 200 {
        let body = response.as_str().unwrap_or_default();
        panic!("Failed to get reports: {} {}", response.status_code, body);
    }

    let body = response.as_str().unwrap_or("[]");
    let mut reports: Vec<Report> = serde_json::from_str(body).expect("Failed to parse reports");
    reports.retain(|report| !report.action_taken);

    let now = time::OffsetDateTime::now_utc();
    for report in reports.iter_mut() {
        let duration = now - report.created_at;
        if duration.whole_hours() > 23 {
            return true;
        }
        println!("{} ago", duration);
    }

    false
}

fn main() {
    let token = env::var("MASTODON_TOKEN").expect("Expected a token in the environment");

    loop {
        if should_shutdown(&token) {
            println!("Server should shut down!");
        }
        
        std::thread::sleep(std::time::Duration::from_secs(60*30));
    }
}
