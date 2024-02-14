use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct Link {
    pub git: Option<String>,
    pub html: Option<String>,
    #[serde(rename = "self")]
    pub self_: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Entry {
    #[serde(rename = "type")]
    pub type_: String,
    pub size: i32,
    pub name: String,
    pub path: String,
    pub sha: String,
    pub url: String,
    pub git_url: Option<String>,
    pub html_url: Option<String>,
    pub download_url: Option<String>,
    #[serde(rename = "_links")]
    pub links: Link,
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
    pub repos: Vec<Repo>,
    pub qdrant: Host,
    pub open_ai: Host,
}
