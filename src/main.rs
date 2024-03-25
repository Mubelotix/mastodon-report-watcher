use std::{env, process::Command, thread::sleep, time::Duration};
use minreq::get;
use serde::Deserialize;
use time::OffsetDateTime;

#[derive(Debug, Deserialize)]
struct Account {
    username: String,
    domain: Option<String>,

    // With more fields that are not relevant here
}

impl Account {
    fn format_username(&self) -> String {
        match &self.domain {
            Some(domain) => format!("@{}@{}", self.username, domain),
            None => format!("@{}", self.username),
        }
    }
}

#[derive(Debug, Deserialize)]
struct Report {
    action_taken: bool,

    category: String,
    comment: String,

    account: Account,
    target_account: Account,

    #[serde(with = "time::serde::iso8601")]
    created_at: OffsetDateTime,

    //  With more fields that are not relevant here
}

#[derive(Debug)]
enum Error {
    Http(minreq::Error),
    Json(serde_json::Error),
    Api { code: i32, body: String },
}

impl From<minreq::Error> for Error {
    fn from(error: minreq::Error) -> Self {
        Error::Http(error)
    }
}

impl From<serde_json::Error> for Error {
    fn from(error: serde_json::Error) -> Self {
        Error::Json(error)
    }
}

fn should_shutdown(token: &str) -> Result<(bool, Option<Report>), Error> {
    let response = get("https://mastodon.insa.lol/api/v1/admin/reports")
        .with_header("Authorization", format!("Bearer {}", token))
        .send()?;

    if response.status_code != 200 {
        let body = response.as_str().unwrap_or_default();
        return Err(Error::Api {
            code: response.status_code,
            body: body.to_string(),
        });
    }

    let body = response.as_str().unwrap_or("[]");
    let mut reports: Vec<Report> = serde_json::from_str(body)?;
    reports.retain(|report| !report.action_taken);

    let now = OffsetDateTime::now_utc();
    for report in reports {
        let duration = now - report.created_at;
        if duration.whole_hours() > 23 {
            return Ok((true, Some(report)));
        }
    }

    Ok((false, None))
}

fn send_webhook(webhook_url: &str, content: &str, title: &str, comment: &str, author: &str, target: &str) {
    let value = format!(r#"{{
        "content": "{content}",
        "embeds": [{{
            "title": "{title}",
            "description": "{comment}",
            "color": 16711680,
            "fields": [
                {{
                    "name": "auteur",
                    "value": "{author}",
                    "inline": true
                }},
                {{
                    "name": "target",
                    "value": "{target}",
                    "inline": true
                }}
            ]
        }}],
        "attachments": []
    }}"#);
    
    let resp = minreq::post(webhook_url)
        .with_header("Content-Type", "application/json")
        .with_body(value)
        .send()
        .expect("Failed to send webhook");
    if resp.status_code != 204 {
        println!("Failed to send webhook: {}", resp.status_code);
    }
}

const SHUTDOWN_MSG: &str = "🚨 Report wasn't processed in due time. Server shutting down to prevent legal issues. <@480708256029736960>";

fn shutdown(webhook_url: &str, service: &str, report: Report) {
    println!("Shutting down");
    send_webhook(webhook_url, SHUTDOWN_MSG, &report.category, &report.comment, &report.account.format_username(), &report.target_account.format_username());
    Command::new("sh")
        .arg("-c")
        .arg(format!("sudo /usr/bin/systemctl stop {service}"))
        .output()
        .expect("Failed to shutdown");
}

fn force_follow(users: &[String]) {
    for user in users {
        Command::new("sh")
            .arg("-c")
            .arg(dbg!(format!("sudo /usr/bin/docker exec mastodon_web_1 /bin/bash -c 'RAILS_ENV=production bin/tootctl accounts follow {user}'")))
            .output()
            .expect("Failed to follow");
    }
}

fn main() {
    let token = env::var("MASTODON_TOKEN").expect("Expected a token in the environment");
    let webhook_url = env::var("WEBHOOK_URL").expect("Expected a webhook url in the environment");
    let service = env::var("MASTODON_SERVICE").ok().unwrap_or_else(|| String::from("mastodon"));
    let users_to_follow: Vec<String> = env::var("USERS_TO_FOLLOW").ok().map(|s| s.split(',').map(|s| s.to_owned()).collect()).unwrap_or_default();

    let mut retries = 0;
    loop {
        match should_shutdown(&token) {
            Ok((should_shutdown, report)) => {
                retries = 0;
                if let Some(report) = report {
                    if should_shutdown {
                        shutdown(&webhook_url, &service, report);
                    }
                }
            },
            Err(e) => {
                println!("Error: {e:?}");
                if retries > 10 {
                    println!("Too many retries.");
                }
                sleep(Duration::from_secs(10));
                retries += 1;
            }
        }

        force_follow(&users_to_follow);
        sleep(Duration::from_secs(60*30));
    }
}
