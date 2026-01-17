use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Movie {
    pub id: i32,
    pub title: String,
    pub tagline: Option<String>,
    pub popularity: Option<f64>,
    pub release_date: Option<String>,
}
