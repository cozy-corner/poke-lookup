#[cfg(feature = "sprites")]
use anyhow::{Context, Result};
#[cfg(feature = "sprites")]
use directories::ProjectDirs;
#[cfg(feature = "sprites")]
use reqwest::blocking::Client;
#[cfg(feature = "sprites")]
use std::collections::HashMap;
#[cfg(feature = "sprites")]
use std::path::{Path, PathBuf};

#[cfg(feature = "sprites")]
pub struct SpriteService {
    cache_dir: PathBuf,
    client: Client,
    base_url: String,
    id_map: HashMap<String, u32>,
}

#[cfg(feature = "sprites")]
impl SpriteService {
    pub fn new() -> Result<Self> {
        use crate::data::DataLoader;

        let project_dirs = ProjectDirs::from("", "", "poke-lookup")
            .or_else(|| ProjectDirs::from("dev", "poke-lookup", "poke-lookup"))
            .context("Failed to determine project directories")?;

        let cache_dir = project_dirs.data_dir().join("sprites");

        if !cache_dir.exists() {
            std::fs::create_dir_all(&cache_dir).with_context(|| {
                format!(
                    "Failed to create sprite cache directory: {}",
                    cache_dir.display()
                )
            })?;
        }

        let client = Client::builder()
            .user_agent("poke-lookup/0.1.0")
            .build()
            .context("Failed to create HTTP client")?;

        // Load Pokemon ID mapping
        let loader = DataLoader::new()?;
        let dictionary = loader.load_dictionary()?;
        let id_map = dictionary
            .entries
            .iter()
            .filter_map(|entry| entry.id.map(|id| (entry.en.clone(), id)))
            .collect();

        Ok(Self {
            cache_dir,
            client,
            base_url: "https://raw.githubusercontent.com/PokeAPI/sprites/master".to_string(),
            id_map,
        })
    }

    pub fn get_pokemon_id(&self, english_name: &str) -> Option<u32> {
        self.id_map.get(english_name).copied()
    }

    pub fn get_sprite_path(&self, pokemon_id: u32) -> PathBuf {
        self.cache_dir.join(format!("{}.png", pokemon_id))
    }

    pub fn display_sprite_for_pokemon(&self, english_name: &str) -> Result<()> {
        if let Some(pokemon_id) = self.get_pokemon_id(english_name) {
            match self.fetch_sprite(pokemon_id) {
                Ok(sprite_path) => {
                    self.display_sprite(&sprite_path)?;
                }
                Err(_) => {
                    // 静かに失敗
                }
            }
        }
        Ok(())
    }

    pub fn fetch_sprite(&self, pokemon_id: u32) -> Result<PathBuf> {
        let sprite_path = self.get_sprite_path(pokemon_id);

        if sprite_path.exists() {
            return Ok(sprite_path);
        }

        let url = format!("{}/sprites/pokemon/{}.png", self.base_url, pokemon_id);

        let response = self
            .client
            .get(&url)
            .send()
            .with_context(|| format!("Failed to fetch sprite for Pokemon ID {}", pokemon_id))?;

        if !response.status().is_success() {
            return Err(anyhow::anyhow!(
                "Failed to download sprite for Pokemon ID {}: HTTP {}",
                pokemon_id,
                response.status()
            ));
        }

        let content = response.bytes().context("Failed to read sprite data")?;

        std::fs::write(&sprite_path, content)
            .with_context(|| format!("Failed to save sprite to {}", sprite_path.display()))?;

        Ok(sprite_path)
    }

    pub fn display_sprite(&self, sprite_path: &Path) -> Result<()> {
        #[cfg(feature = "sprites")]
        {
            // Try viuer first - it handles terminal detection automatically
            let img = image::open(sprite_path).with_context(|| {
                format!("Failed to open sprite image: {}", sprite_path.display())
            })?;

            let config = viuer::Config {
                transparent: true,
                absolute_offset: false,
                ..Default::default()
            };

            match viuer::print(&img, &config) {
                Ok(_) => {}
                Err(e) => {
                    // Fallback to text if viuer fails
                    println!("🖼️  Sprite saved at: {}", sprite_path.display());
                    println!("   (Terminal image display not available: {})", e);
                }
            }
        }
        Ok(())
    }

    #[cfg(test)]
    pub fn with_base_url(cache_dir: PathBuf, client: Client, base_url: String) -> Self {
        Self {
            cache_dir,
            client,
            base_url,
            id_map: HashMap::new(),
        }
    }

    #[cfg(test)]
    pub fn for_test(cache_dir: PathBuf, id_map: HashMap<String, u32>) -> Self {
        Self {
            cache_dir,
            client: Client::new(),
            base_url: "test://mock".to_string(),
            id_map,
        }
    }
}

#[cfg(test)]
#[cfg(feature = "sprites")]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn test_for_test_creates_service() {
        // Test that for_test() creates a SpriteService successfully
        let temp_dir = tempdir().unwrap();
        let id_map = HashMap::new();

        let service = SpriteService::for_test(temp_dir.path().to_path_buf(), id_map);
        assert_eq!(service.base_url, "test://mock");
    }

    #[test]
    fn test_sprite_path() {
        let temp_dir = tempdir().unwrap();
        let service = SpriteService {
            cache_dir: temp_dir.path().to_path_buf(),
            client: Client::new(),
            base_url: "http://dummy.example.com".to_string(),
            id_map: HashMap::new(),
        };

        let path = service.get_sprite_path(25);
        assert_eq!(path, temp_dir.path().join("25.png"));

        let path = service.get_sprite_path(1);
        assert_eq!(path, temp_dir.path().join("1.png"));
    }

    #[test]
    fn test_has_cached_sprite() {
        let temp_dir = tempdir().unwrap();
        let service = SpriteService {
            cache_dir: temp_dir.path().to_path_buf(),
            client: Client::new(),
            base_url: "http://dummy.example.com".to_string(),
            id_map: HashMap::new(),
        };

        let sprite_path = service.get_sprite_path(25);
        assert!(!sprite_path.exists());

        fs::write(&sprite_path, b"dummy").unwrap();
        assert!(sprite_path.exists());

        let other_sprite_path = service.get_sprite_path(26);
        assert!(!other_sprite_path.exists());
    }

    #[test]
    fn test_cache_dir() {
        let temp_dir = tempdir().unwrap();
        let cache_path = temp_dir.path().to_path_buf();
        let service = SpriteService {
            cache_dir: cache_path.clone(),
            client: Client::new(),
            base_url: "http://dummy.example.com".to_string(),
            id_map: HashMap::new(),
        };

        // Test through get_sprite_path which uses cache_dir
        let sprite_path = service.get_sprite_path(1);
        assert!(sprite_path.starts_with(&cache_path));
    }

    #[test]
    fn test_fetch_sprite_cached() {
        let temp_dir = tempdir().unwrap();
        let service = SpriteService {
            cache_dir: temp_dir.path().to_path_buf(),
            client: Client::new(),
            base_url: "http://dummy.example.com".to_string(),
            id_map: HashMap::new(),
        };

        // Create a cached sprite
        let sprite_path = service.get_sprite_path(25);
        fs::write(&sprite_path, b"cached_image").unwrap();

        // Fetch should return the cached path without downloading
        let result = service.fetch_sprite(25);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), sprite_path);

        // Verify content wasn't changed
        let content = fs::read(&sprite_path).unwrap();
        assert_eq!(content, b"cached_image");
    }

    #[test]
    fn test_display_sprite_file_not_found() {
        let temp_dir = tempdir().unwrap();
        let service = SpriteService {
            cache_dir: temp_dir.path().to_path_buf(),
            client: Client::new(),
            base_url: "http://dummy.example.com".to_string(),
            id_map: HashMap::new(),
        };

        let non_existent = temp_dir.path().join("non_existent.png");
        let result = service.display_sprite(&non_existent);
        assert!(result.is_err());
    }

    #[test]
    fn test_fetch_sprite_download_success() {
        use httpmock::prelude::*;

        // Start a mock server
        let server = MockServer::start();

        // Create a mock sprite image (1x1 PNG)
        let mock_png = vec![
            0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, // PNG signature
            0x00, 0x00, 0x00, 0x0D, 0x49, 0x48, 0x44, 0x52, // IHDR chunk
            0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x01, 0x00, 0x00, 0x00, 0x00, 0x37,
            0x6E, 0xF9, 0x24, 0x00, 0x00, 0x00, 0x0A, 0x49, 0x44, 0x41, 0x54, 0x78, 0x9C, 0x62,
            0x00, 0x00, 0x00, 0x00, 0x02, 0x00, 0x01, 0xE5, 0x27, 0xDE, 0xFC, 0x00, 0x00, 0x00,
            0x00, 0x49, 0x45, 0x4E, 0x44, 0xAE, 0x42, 0x60, 0x82,
        ];

        // Create a mock for Pokemon sprite
        let mock = server.mock(|when, then| {
            when.method(GET).path("/sprites/pokemon/25.png");
            then.status(200)
                .header("content-type", "image/png")
                .body(&mock_png);
        });

        // Create service with mock server URL
        let temp_dir = tempdir().unwrap();
        let client = Client::builder()
            .user_agent("poke-lookup/0.1.0")
            .build()
            .unwrap();

        let service =
            SpriteService::with_base_url(temp_dir.path().to_path_buf(), client, server.url(""));

        // Test fetching sprite
        let result = service.fetch_sprite(25);
        assert!(result.is_ok());

        let sprite_path = result.unwrap();
        assert!(sprite_path.exists());

        // Verify the downloaded content
        let content = fs::read(&sprite_path).unwrap();
        assert_eq!(content, mock_png);

        mock.assert();
    }

    #[test]
    fn test_fetch_sprite_download_failure() {
        use httpmock::prelude::*;

        let server = MockServer::start();

        // Create a mock that returns 404
        let _mock = server.mock(|when, then| {
            when.method(GET).path("/sprites/pokemon/9999.png");
            then.status(404)
                .header("content-type", "text/html")
                .body("Not Found");
        });

        let temp_dir = tempdir().unwrap();
        let service = SpriteService::with_base_url(
            temp_dir.path().to_path_buf(),
            Client::new(),
            server.url(""),
        );

        // Test fetching sprite that doesn't exist
        let result = service.fetch_sprite(9999);
        assert!(result.is_err());

        // Verify that no file was created
        assert!(!service.get_sprite_path(9999).exists());
    }

    #[test]
    fn test_get_pokemon_id() {
        let temp_dir = tempdir().unwrap();
        let mut id_map = HashMap::new();
        id_map.insert("Pikachu".to_string(), 25);
        id_map.insert("Bulbasaur".to_string(), 1);

        let service = SpriteService::for_test(temp_dir.path().to_path_buf(), id_map);

        assert_eq!(service.get_pokemon_id("Pikachu"), Some(25));
        assert_eq!(service.get_pokemon_id("Bulbasaur"), Some(1));
        assert_eq!(service.get_pokemon_id("Unknown"), None);
    }
}
