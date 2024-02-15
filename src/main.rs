extern crate llm_chain;
extern crate llm_chain_openai;

mod models;
mod config;
mod utils;

use config::read_config;
use utils::{fetch_content_tree, download_markdown_files};
use reqwest::header::{HeaderMap, HeaderValue, USER_AGENT};
use llm_chain::{schema::EmptyMetadata, schema::Document, traits::VectorStore};
use llm_chain_qdrant::Qdrant;
use qdrant_client::{
    prelude::{QdrantClient, QdrantClientConfig},
    qdrant::{CreateCollection, Distance, VectorParams, VectorsConfig},
};

use async_openai::{Client, config::OpenAIConfig};


use std::sync::Arc;
use std::fs;
use std::io;



fn read_file_to_string(file_path: &str) -> io::Result<String> {
    fs::read_to_string(file_path)
}

fn create_document_from_file(file_path: &str) -> io::Result<Document<EmptyMetadata>> {
    let page_content = read_file_to_string(file_path)?;
    Ok(Document::new(page_content))
}

#[tokio::main(flavor = "current_thread")]
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
    std::env::set_var("OPENAI_API_BASE", format!("http://{}:{}/{}", 
        config.open_ai.host, 
        config.open_ai.port, 
        config.open_ai.path
            .unwrap_or("".to_string())
    ));

    std::env::set_var("OPENAI_BETA_HEADER", "assistants=v1");

    std::env::set_var("OPENAI_API_KEY", "ollama");
    let openai_config = OpenAIConfig::new().with_api_base("http://10.0.1.1:11434/v1".to_string());
    let client = Client::with_config(openai_config.with_api_key("ollama".to_string()));
    println!("{:#?}", client);
    
    
    let qurl = format!("http://{}:{}", config.qdrant.host, config.qdrant.port);
    let qconfig = QdrantClientConfig::from_url("http://localhost:6334");
    let qclient = Arc::new(QdrantClient::new(Some(qconfig)).unwrap());
    let collection_name = "aws_test".to_string();
    let embedding_size = 1536;

    if !qclient.has_collection(collection_name.clone()).await.unwrap()
    {
        qclient.create_collection(&CreateCollection {
            collection_name: collection_name.clone(),
            vectors_config: Some(VectorsConfig {
                config: Some(qdrant_client::qdrant::vectors_config::Config::Params(
                VectorParams {
                    on_disk: None,
                        size: embedding_size,
                        distance: Distance::Cosine.into(),
                        hnsw_config: None,
                        quantization_config: None,
                    },
                )),
            }),
            ..Default::default()
        })
        .await
        .unwrap();
    };

    let embeddings = llm_chain_openai::embeddings::Embeddings::for_client(client, "codellama");

    // Storing documents
    let qdrant: Qdrant<llm_chain_openai::embeddings::Embeddings, EmptyMetadata> = Qdrant::new(
        qclient.clone(),
        collection_name.clone(),
        embeddings,
        None,
        None,
        None,
    );
    let file_path = "./markdown/region.html.markdown";
    let document = create_document_from_file(file_path).expect("Failed to fetch document");
    let doc_ids = qdrant.add_texts(vec!["all your bases are belong to us".to_string()]).await.unwrap();

    println!("Vectors stored under IDs {:?}", doc_ids);

    let response = qclient.get_points(
        collection_name, 
        None, 
        &doc_ids.into_iter().map(|id| id.into()).collect::<Vec<_>>(), 
        Some(true), 
        Some(true), 
        None,
    )
    .await
    .unwrap();

    println!("Retrieved stored vectors: {:?}", response.result)

}
//    let opts = options!(
//         Model: ModelRef::from_model_name("codellama")
//    );
//    let exec = executor!(chatgpt, opts.clone()).expect("Failed to create executor");
//    let query = r#"How do I use the "match" keyword in Rust?"#;
//    println!("Query: {query}\n");
//    let res = prompt!("", query,).run(&parameters!(), &exec).await;
//    match res {
//        Ok(res) => println!("AI:\n{res}"),
//        Err(e) => println!("Error: {e}"),
//    }
