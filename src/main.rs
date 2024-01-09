use std::{env, thread::sleep, time::Duration};
use minreq::get;
use serde::Deserialize;
use time::OffsetDateTime;

#[derive(Debug, Deserialize)]
struct Report {
    action_taken: bool,
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

fn should_shutdown(token: &str) -> Result<bool, Error> {
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
    for report in reports.iter_mut() {
        let duration = now - report.created_at;
        if duration.whole_hours() > 23 {
            return Ok(true);
        }
    }

    Ok(false)
}

fn shutdown() {
    println!("Shutting down");
}

fn main() {
    let token = env::var("MASTODON_TOKEN").expect("Expected a token in the environment");

    let mut retries = 0;
    loop {
        match should_shutdown(&token) {
            Ok(should_shutdown) => {
                retries = 0;
                if should_shutdown {
                    shutdown();
                }
            },
            Err(e) => {
                println!("Error: {e:?}");
                if retries > 10 {
                    println!("Too many retries.");
                    shutdown()
                }
                sleep(Duration::from_secs(10));
                retries += 1;
            }
        }

        sleep(Duration::from_secs(60*30));
    }
}
