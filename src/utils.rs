use crate::models::Entry;
use reqwest::header::HeaderMap;
use indicatif::{ProgressBar, ProgressStyle};
use anyhow::{Result, Error};
use std::fs::File;
use std::io::Write;
use futures::future::try_join_all;
use tokio::task;

pub async fn fetch_content_tree(url: &str, bearer_token: &String, headers: &HeaderMap) -> Result<Vec<Entry>> {
    let client = reqwest::Client::new();
    let response = client.get(url)
        .headers(headers.clone())
        .bearer_auth(&bearer_token)
        .send()
        .await?;
    let entries = response
        .json::<Vec<Entry>>()
        .await?;
    Ok(entries)
}

pub async fn download_markdown_files(entries: Vec<Entry>, file_path: String, bearer_token: &String, headers: &HeaderMap) -> Result<()> {
    let client = reqwest::Client::new();

    // Create a new progress bar with a length of the total number of entries
    let progress_bar = ProgressBar::new(entries.len() as u64);

    progress_bar.set_style(ProgressStyle::default_bar()
        .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos:>7}/{len:7} {msg}")
        .expect("Error setting progress bar template")
        .progress_chars("##-"));

    let tasks: Vec<_> = entries.into_iter().filter(|e| e.name.ends_with(".markdown")).map(|entry| {
        let client = client.clone();
        let headers = headers.clone();
        let bearer_token = bearer_token.clone();
        let file_path = file_path.clone();
        let progress_bar = progress_bar.clone();

        task::spawn(async move {
            if let Some(download_url) = entry.download_url {
                let content = client.get(&download_url).headers(headers).bearer_auth(&bearer_token).send().await?.text().await?;
                let full_file_path = format!("{}/{}", file_path, entry.name);
                let mut file = File::create(full_file_path)?;
                file.write_all(content.as_bytes())?;

                progress_bar.inc(1);
                progress_bar.set_message(entry.name);
            }
            Ok::<(), Error>(())
        })
    }).collect();

    // Wait for all tasks to complete.
    try_join_all(tasks).await?;

    progress_bar.finish_with_message("Download complete");

    Ok(())
}
