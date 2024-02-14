extern crate llm_chain;
extern crate llm_chain_openai;

mod models;
mod config;
mod utils;

use config::Read_Config;
use utils::{Fetch_Content_Tree, Download_Markdown_Files};
use reqwest::header::{HeaderMap, HeaderValue, USER_AGENT};
use llm_chain::options;
use llm_chain::options::ModelRef;
use llm_chain::{executor, parameters, prompt};







#[tokio::main]
async fn main() {
    // Load the config
    let config_path = "config.toml";
    let config = Read_Config(config_path);

    // Set the save path
    let file_path = String::from("markdown");

    // Control the needed headers
    let mut headers = HeaderMap::new();
    headers.insert(USER_AGENT, HeaderValue::from_static("Rust"));
    
    // Get the bearer_token from env
    let bearer_token = std::env::var("GITHUB_BEARER_TOKEN").expect("GITHUB_BEARER_TOKEN not set");

    for repo in config.repos.iter() {
    let url = format!("https://api.github.com/repos/{}/{}/contents/{}", repo.owner, repo.name, repo.path);
        match Fetch_Content_Tree(&url, &bearer_token, &headers).await {
            Ok(entries) => {
                if let Err(e) = Download_Markdown_Files(entries, file_path.clone(), &bearer_token, &headers).await {
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

