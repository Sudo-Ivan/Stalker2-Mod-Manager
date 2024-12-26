use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION};
use serde::Deserialize;
use anyhow::Result;
use url;

const NEXUS_API_BASE: &str = "https://api.nexusmods.com/v1";
const GAME_DOMAIN: &str = "stalker2heartofchornobyl";

#[derive(Debug, Deserialize)]
pub struct NexusModInfo {
    pub name: String,
    pub version: Option<String>,
    pub author: String,
    #[serde(default)]
    pub description: String,
    #[serde(rename = "mod_id")]
    pub id: i32,
    pub category_id: Option<i32>,
    pub status: String,
    pub available: bool,
    pub user: ModUser,
}

#[derive(Debug, Deserialize)]
pub struct ModUser {
    pub name: String,
    pub member_id: i32,
}

#[derive(Debug, Deserialize)]
pub struct ModFile {
    #[serde(rename = "id")]
    id_array: Vec<i32>,
    pub name: String,
    pub version: Option<String>,
    pub category_id: Option<i32>,
    pub file_name: String,
    pub mod_version: Option<String>,
}

impl ModFile {
    pub fn id(&self) -> i32 {
        self.id_array[0] // Get the first ID from the array
    }
}

#[derive(Debug)]
pub struct NxmLink {
    pub game_domain: String,
    pub mod_id: i32,
    pub file_id: i32,
    pub key: String,
    pub expires: i64,
}

impl NxmLink {
    pub fn parse(nxm_url: &str) -> Result<Self> {
        // Format: nxm://stalker2heartofchornobyl/mods/33/files/130?key=xxx&expires=1234567890
        let url = url::Url::parse(nxm_url)?;
        
        if url.scheme() != "nxm" {
            return Err(anyhow::anyhow!("Invalid NXM URL scheme"));
        }

        let segments: Vec<&str> = url.path_segments()
            .ok_or_else(|| anyhow::anyhow!("Invalid URL path"))?
            .collect();
        
        if segments.len() != 4 {
            return Err(anyhow::anyhow!("Invalid NXM URL format"));
        }

        let query: std::collections::HashMap<_, _> = url.query_pairs().collect();
        
        Ok(Self {
            game_domain: url.host_str()
                .ok_or_else(|| anyhow::anyhow!("Missing game domain"))?
                .to_string(),
            mod_id: segments[1].parse()?,
            file_id: segments[3].parse()?,
            key: query.get("key")
                .ok_or_else(|| anyhow::anyhow!("Missing key"))?
                .to_string(),
            expires: query.get("expires")
                .ok_or_else(|| anyhow::anyhow!("Missing expires"))?
                .parse()?,
        })
    }
}

#[derive(Debug, Deserialize)]
pub struct DownloadLink {
    pub name: String,
    pub short_name: String,
    #[serde(rename = "URI")]
    pub uri: String,
}

#[derive(Debug, Deserialize)]
pub struct ModFilesResponse {
    pub files: Vec<ModFile>,
}

pub struct NexusClient {
    client: reqwest::Client,
    api_key: String,
}

impl NexusClient {
    pub fn new(api_key: &str) -> Result<Self> {
        let mut headers = HeaderMap::new();
        headers.insert(
            AUTHORIZATION, 
            HeaderValue::from_str(&format!("Bearer {}", api_key))?
        );
        headers.insert("apikey", HeaderValue::from_str(api_key)?);
        
        let client = reqwest::Client::builder()
            .default_headers(headers)
            .build()?;
            
        Ok(Self { 
            client,
            api_key: api_key.to_string(),
        })
    }

    pub async fn get_mod_info(&self, mod_id: i32) -> Result<NexusModInfo> {
        let url = format!("{}/games/{}/mods/{}", NEXUS_API_BASE, GAME_DOMAIN, mod_id);
        let response = self.client.get(&url).send().await?;
        
        if !response.status().is_success() {
            return Err(anyhow::anyhow!("Failed to get mod info: {}", response.status()));
        }
        
        // Print the response body for debugging
        let text = response.text().await?;
        println!("API Response for mod info: {}", text);
        
        // Parse the JSON text
        let mod_info = serde_json::from_str(&text)?;
        Ok(mod_info)
    }

    pub async fn get_mod_files(&self, mod_id: i32) -> Result<Vec<ModFile>> {
        let url = format!("{}/games/{}/mods/{}/files", NEXUS_API_BASE, GAME_DOMAIN, mod_id);
        let response = self.client.get(&url).send().await?;
        
        if !response.status().is_success() {
            return Err(anyhow::anyhow!("Failed to get mod files: {}", response.status()));
        }
        
        // Print the response body for debugging
        let text = response.text().await?;
        println!("API Response for mod files: {}", text);
        
        // Parse the JSON text
        let files_response: ModFilesResponse = serde_json::from_str(&text)?;
        Ok(files_response.files)
    }

    pub async fn download_mod(&self, mod_id: i32, file_id: i32, nxm_info: Option<(String, i64)>) -> Result<Vec<u8>> {
        let url = format!(
            "{}/games/{}/mods/{}/files/{}/download_link.json",
            NEXUS_API_BASE, GAME_DOMAIN, mod_id, file_id
        );
        
        let mut query = Vec::new();
        if let Some((key, expires)) = nxm_info {
            query.push(("key", key));
            query.push(("expires", expires.to_string()));
        }
        
        let response = self.client.get(&url)
            .query(&query)
            .header("apikey", &self.api_key)
            .header("accept", "application/json")
            .send()
            .await?;
        
        if response.status() == reqwest::StatusCode::FORBIDDEN {
            return Err(anyhow::anyhow!(
                "Access denied. Premium Nexus account required for API downloads. \
                Please download manually from the website or upgrade your account."
            ));
        }
        
        if !response.status().is_success() {
            return Err(anyhow::anyhow!("Failed to get download link: {}", response.status()));
        }
        
        // Print response for debugging
        let text = response.text().await?;
        println!("Download link response: {}", text);
        
        let download_links: Vec<DownloadLink> = serde_json::from_str(&text)?;
        let download_url = download_links.first()
            .ok_or_else(|| anyhow::anyhow!("No download links available"))?;
        
        // Download the actual file
        let mod_response = self.client.get(&download_url.uri)
        .header("apikey", &self.api_key)
        .send()
        .await?;
        
        if !mod_response.status().is_success() {
            return Err(anyhow::anyhow!("Failed to download mod: {}", mod_response.status()));
        }
        
        let mod_data = mod_response.bytes().await?;
        Ok(mod_data.to_vec())
    }
} 