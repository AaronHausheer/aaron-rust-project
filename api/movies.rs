use hyper::body::to_bytes;
use hyper::Method;
use serde::Deserialize;
use serde_json::{json, Value}; // JSON macro and type live here
use std::env;
use std::string::String;
use vercel_runtime::{run, service_fn, Error, Request};

#[path = "../src/movie.rs"]
mod movie;
use movie::Movie;

#[derive(Debug, Deserialize)]
struct MovieInput {
    title: String,
    tagline: Option<String>,
    popularity: Option<f64>,
    release_date: Option<String>,
}

#[derive(Debug, Deserialize)]
struct MovieUpdate {
    title: Option<String>,
    tagline: Option<String>,
    popularity: Option<f64>,
    release_date: Option<String>,
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    // Official Vercel v2 initialization
    run(service_fn(handler)).await
}

pub async fn handler(req: Request) -> Result<Value, Error> {
    let supabase_url = env::var("SUPABASE_URL").unwrap_or_default();
    let supabase_key = env::var("SUPABASE_ANON_KEY").unwrap_or_default();

/******** DEBUG code to check variables ********/
    println!("ENV CHECK - URL length: {}, Key length: {}", supabase_url.len(), supabase_key.len());

    if supabase_url.is_empty() || supabase_key.is_empty() {
        return Ok(json!({
            "error": "Backend environment variables are not set",
            "details": "Check Vercel Dashboard > Settings > Environment Variables"
        }));
    }

    // This will show up in Vercel 'Logs' but won't reveal your secret
    if supabase_key.is_empty() {
        eprintln!("CRITICAL: SUPABASE_ANON_KEY is empty on the server!");
    } else {
        println!("SUCCESS: SUPABASE_ANON_KEY detected (Length: {})", supabase_key.len());
    }


    // Query parsing
    let uri_string = req.uri().to_string();
    let query_parts: std::collections::HashMap<String, String> = uri_string
        .split('?')
        .nth(1)
        .unwrap_or("")
        .split('&')
        .filter(|s| !s.is_empty())
        .map(|s| {
            let mut parts = s.split('=');
            (
                parts.next().unwrap_or("").to_string(),
                parts.next().unwrap_or("").to_string(),
            )
        })
        .collect();

    let client = reqwest::Client::new();

    match *req.method() {
        Method::GET => {
            let search_term = query_parts.get("query").cloned().unwrap_or_default();
            let page: usize = query_parts.get("page").and_then(|p| p.parse().ok()).unwrap_or(0);

            let items_per_page = 8;
            let from = page * items_per_page;
            let to = from + items_per_page - 1;

            let mut target_url = format!("{}/rest/v1/movies?select=*", supabase_url);
            if !search_term.is_empty() {
                target_url.push_str(&format!("&title=ilike.*{}*", search_term));
            }
            target_url.push_str("&order=release_date.desc");

            let res = client
                .get(target_url)
                .header("apikey", &supabase_key)
                .header("Authorization", format!("Bearer {}", supabase_key))
                .header("Range", format!("{}-{}", from, to))
                .header("Prefer", "count=exact")
                .send()
                .await?;

            let total_count = res
                .headers()
                .get("content-range")
                .and_then(|v| v.to_str().ok())
                .and_then(|v| v.split('/').last())
                .map(|v| v.to_string())
                .unwrap_or_else(|| "0".to_string());

            let movies: Vec<Movie> = res.json().await?;

            Ok(json!({
                "movies": movies,
                "total": total_count.parse::<usize>().unwrap_or(0)
            }))
        }
        Method::POST => {
            let body_bytes = to_bytes(req.into_body()).await?;
            let payload: MovieInput = match serde_json::from_slice(&body_bytes) {
                Ok(data) => data,
                Err(_) => {
                    return Ok(json!({
                        "error": "Invalid JSON payload."
                    }));
                }
            };

            let target_url = format!("{}/rest/v1/movies", supabase_url);
            let res = client
                .post(target_url)
                .header("apikey", &supabase_key)
                .header("Authorization", format!("Bearer {}", supabase_key))
                .header("Prefer", "return=representation")
                .json(&payload)
                .send()
                .await?;

            if !res.status().is_success() {
                let details = res.text().await.unwrap_or_else(|_| "Unknown error".to_string());
                return Ok(json!({
                    "error": "Supabase insert failed.",
                    "details": details
                }));
            }

            let created: Vec<Movie> = res.json().await.unwrap_or_default();
            Ok(json!({
                "movie": created.into_iter().next()
            }))
        }
        Method::PATCH => {
            let id = match query_parts.get("id") {
                Some(value) if !value.is_empty() => value,
                _ => {
                    return Ok(json!({
                        "error": "Missing id query parameter."
                    }));
                }
            };

            let body_bytes = to_bytes(req.into_body()).await?;
            let payload: MovieUpdate = match serde_json::from_slice(&body_bytes) {
                Ok(data) => data,
                Err(_) => {
                    return Ok(json!({
                        "error": "Invalid JSON payload."
                    }));
                }
            };

            if payload.title.is_none()
                && payload.tagline.is_none()
                && payload.popularity.is_none()
                && payload.release_date.is_none()
            {
                return Ok(json!({
                    "error": "No fields provided for update."
                }));
            }

            let target_url = format!("{}/rest/v1/movies?id=eq.{}", supabase_url, id);
            let res = client
                .patch(target_url)
                .header("apikey", &supabase_key)
                .header("Authorization", format!("Bearer {}", supabase_key))
                .header("Prefer", "return=representation")
                .json(&payload)
                .send()
                .await?;

            if !res.status().is_success() {
                let details = res.text().await.unwrap_or_else(|_| "Unknown error".to_string());
                return Ok(json!({
                    "error": "Supabase update failed.",
                    "details": details
                }));
            }

            let updated: Vec<Movie> = res.json().await.unwrap_or_default();
            Ok(json!({
                "movie": updated.into_iter().next()
            }))
        }
        Method::DELETE => {
            let id = match query_parts.get("id") {
                Some(value) if !value.is_empty() => value,
                _ => {
                    return Ok(json!({
                        "error": "Missing id query parameter."
                    }));
                }
            };

            let target_url = format!("{}/rest/v1/movies?id=eq.{}", supabase_url, id);
            let res = client
                .delete(target_url)
                .header("apikey", &supabase_key)
                .header("Authorization", format!("Bearer {}", supabase_key))
                .send()
                .await?;

            if !res.status().is_success() {
                let details = res.text().await.unwrap_or_else(|_| "Unknown error".to_string());
                return Ok(json!({
                    "error": "Supabase delete failed.",
                    "details": details
                }));
            }

            Ok(json!({
                "status": "deleted"
            }))
        }
        _ => Ok(json!({
            "error": "Unsupported method."
        })),
    }
}
