use anyhow::{Context, Result};
use reqwest::blocking::Client;
use std::fs;
use std::io::Write;
use std::path::PathBuf;

use crate::data::DataLoader;
use crate::models::NameDictionary;

const DEFAULT_DOWNLOAD_URL: &str =
    "https://github.com/cozy-corner/poke-lookup/releases/latest/download/names.json";

pub struct UpdateService {
    data_loader: DataLoader,
    client: Client,
}

impl UpdateService {
    pub fn new() -> Result<Self> {
        let data_loader = DataLoader::new()?;
        let client = Client::builder()
            .user_agent("poke-lookup/0.1.0")
            .build()
            .context("Failed to create HTTP client")?;

        Ok(Self {
            data_loader,
            client,
        })
    }

    pub fn with_path(dict_path: PathBuf) -> Result<Self> {
        let data_loader = DataLoader::with_path(dict_path);
        let client = Client::builder()
            .user_agent("poke-lookup/0.1.0")
            .build()
            .context("Failed to create HTTP client")?;

        Ok(Self {
            data_loader,
            client,
        })
    }

    pub fn update(&self, source_url: Option<String>, dry_run: bool) -> Result<()> {
        let url = source_url.as_deref().unwrap_or(DEFAULT_DOWNLOAD_URL);

        eprintln!("Downloading from: {}", url);

        let response = self.client
            .get(url)
            .send()
            .context("Failed to send HTTP request")?;

        if !response.status().is_success() {
            return Err(anyhow::anyhow!(
                "Failed to download: HTTP {} {}",
                response.status().as_u16(),
                response.status().canonical_reason().unwrap_or("Unknown")
            ));
        }

        let content = response.bytes()
            .context("Failed to read response body")?;

        let dictionary: NameDictionary = serde_json::from_slice(&content)
            .context("Failed to parse JSON")?;

        eprintln!("Downloaded {} entries", dictionary.count);
        eprintln!("Schema version: {}", dictionary.schema_version);
        eprintln!("Generated at: {}", dictionary.generated_at);

        dictionary.validate()
            .map_err(|e| anyhow::anyhow!("Validation failed: {}", e))?;

        if dry_run {
            eprintln!("Dry run mode: not saving the file");
            return Ok(());
        }

        self.save_atomic(&content)?;

        eprintln!("Successfully updated names.json");
        Ok(())
    }

    fn save_atomic(&self, content: &[u8]) -> Result<()> {
        self.data_loader.ensure_data_dir()?;

        let data_path = self.data_loader.data_path();
        let temp_path = data_path.with_extension("tmp");

        let mut temp_file = fs::File::create(&temp_path)
            .with_context(|| format!("Failed to create temp file: {}", temp_path.display()))?;

        temp_file.write_all(content)
            .with_context(|| format!("Failed to write to temp file: {}", temp_path.display()))?;

        temp_file.sync_all()
            .context("Failed to sync temp file")?;

        fs::rename(&temp_path, &data_path)
            .with_context(|| format!(
                "Failed to rename {} to {}",
                temp_path.display(),
                data_path.display()
            ))?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    use chrono::Utc;
    use crate::models::NameEntry;

    #[test]
    fn test_update_service_creation() {
        let temp_dir = tempdir().unwrap();
        let dict_path = temp_dir.path().join("names.json");

        let service = UpdateService::with_path(dict_path.clone());
        assert!(service.is_ok());
    }

    #[test]
    fn test_save_atomic() {
        let temp_dir = tempdir().unwrap();
        let dict_path = temp_dir.path().join("names.json");

        let service = UpdateService::with_path(dict_path.clone()).unwrap();

        let test_dict = NameDictionary {
            schema_version: 1,
            generated_at: Utc::now(),
            count: 1,
            entries: vec![NameEntry {
                ja: "ピカチュウ".to_string(),
                en: "Pikachu".to_string(),
            }],
        };

        let content = serde_json::to_vec(&test_dict).unwrap();
        let result = service.save_atomic(&content);

        assert!(result.is_ok());
        assert!(dict_path.exists());

        let saved_content = fs::read(&dict_path).unwrap();
        let saved_dict: NameDictionary = serde_json::from_slice(&saved_content).unwrap();
        assert_eq!(saved_dict.count, 1);
        assert_eq!(saved_dict.entries[0].ja, "ピカチュウ");
    }

    #[test]
    fn test_default_url_constant() {
        assert!(DEFAULT_DOWNLOAD_URL.starts_with("https://"));
        assert!(DEFAULT_DOWNLOAD_URL.contains("names.json"));
    }
}