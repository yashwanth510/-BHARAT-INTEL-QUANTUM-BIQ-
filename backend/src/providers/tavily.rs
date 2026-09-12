use anyhow::Result;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct TavilyResult {
    pub title: String,
    pub url: String,
    pub snippet: String,
}

pub async fn search(
    client: &reqwest::Client,
    api_key: &str,
    query: &str,
    max_results: u32,
) -> Result<Vec<TavilyResult>> {
    let body = serde_json::json!({
        "api_key": api_key,
        "query": query,
        "search_depth": "basic",
        "max_results": max_results,
        "include_answer": false
    });

    let resp = client
        .post("https://api.tavily.com/search")
        .json(&body)
        .send()
        .await?;

    let resp = resp.error_for_status()?;

    #[derive(Deserialize)]
    struct TavilyResp {
        results: Vec<TavilyItem>,
    }
    #[derive(Deserialize)]
    struct TavilyItem {
        title: String,
        url: String,
        #[serde(default)]
        content: String,
    }

    let data: TavilyResp = resp.json().await?;
    Ok(data
        .results
        .into_iter()
        .map(|r| TavilyResult {
            title: r.title,
            url: r.url,
            snippet: r.content,
        })
        .collect())
}
