extern crate llm_chain;
extern crate llm_chain_openai;

use indicatif::{ProgressBar, ProgressStyle};
use serde::{Deserialize, Serialize};
use reqwest::header::{HeaderMap, HeaderValue, USER_AGENT};
use std::fs;
use std::fs::File;
use std::io::Write;
use futures::future::try_join_all;
use tokio::task;
use anyhow::{Result, Error};
use toml;
use llm_chain::options;
use llm_chain::options::ModelRef;
use llm_chain::{executor, parameters, prompt};


#[derive(Serialize, Deserialize, Debug)]
struct Link {
    git: Option<String>,
    html: Option<String>,
    #[serde(rename = "self")]
    self_: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct Entry {
    #[serde(rename = "type")]
    type_: String,
    size: i32,
    name: String,
    path: String,
    sha: String,
    url: String,
    git_url: Option<String>,
    html_url: Option<String>,
    download_url: Option<String>,
    #[serde(rename = "_links")]
    links: Link,
}

#[derive(Debug, Deserialize)]
pub struct Repo {
    pub owner: String,
    pub name: String,
    pub path: String,
    pub tag: String,
}

#[derive(Debug, Deserialize)]
pub struct Host {
    pub host: String,
    pub port: String,
    pub path: Option<String>,
    
}

#[derive(Debug, Deserialize)]
pub struct Config {
    repos: Vec<Repo>,
    qdrant: Host,
    open_ai: Host,
}

async fn fetch_content_tree(url: &str, bearer_token: &String, headers: &HeaderMap) -> Result<Vec<Entry>> {
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

async fn download_markdown_files(entries: Vec<Entry>, file_path: String, bearer_token: &String, headers: &HeaderMap) -> Result<()> {
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

fn read_config(config_path: &str) -> Config {
    let config_content = fs::read_to_string(config_path)
        .expect("Failed to read config file.");
    let config: Config = toml::from_str(&config_content)
        .expect("Failed to parse config file");
    println!("Loaded repo configurations");
    config
}

#[tokio::main]
async fn main() {
    // Load the config
    let config_path = "config.toml";
    let config = read_config(config_path);

    // Set the save path
    let file_path = String::from("markdown");

    // Control the needed headers
    let mut headers = HeaderMap::new();
    headers.insert(USER_AGENT, HeaderValue::from_static("Rust"));
    
    // Get the bearer_token from env
    let bearer_token = std::env::var("GITHUB_BEARER_TOKEN").expect("GITHUB_BEARER_TOKEN not set");

    for repo in config.repos.iter() {
    let url = format!("https://api.github.com/repos/{}/{}/contents/{}", repo.owner, repo.name, repo.path);
        match fetch_content_tree(&url, &bearer_token, &headers).await {
            Ok(entries) => {
                if let Err(e) = download_markdown_files(entries, file_path.clone(), &bearer_token, &headers).await {
                    println!("Error downloading markdown files: {}", e);
                }
            },
            Err(e) => println!("Error fetching content tree: {}", e),
        }
    }
    // This is just running a basic ollama codellama setup calling the bot
    std::env::set_var("OPENAI_API_BASE_URL", format!("http://{}:{}/{}", 
        config.open_ai.host, 
        config.open_ai.port, 
        config.open_ai.path
            .unwrap_or("".to_string())
    ));
    std::env::set_var("OPENAI_API_KEY", "ollama");
    
    let opts = options!(
         Model: ModelRef::from_model_name("codellama")
    );
    let exec = executor!(chatgpt, opts.clone()).expect("Failed to create executor");
    let query = r#"How do I use the "match" keyword in Rust?"#;
    println!("Query: {query}\n");
    let res = prompt!("", query,).run(&parameters!(), &exec).await;
    match res {
        Ok(res) => println!("AI:\n{res}"),
        Err(e) => println!("Error: {e}"),
    }



}

