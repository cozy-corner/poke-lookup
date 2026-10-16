#[cfg(feature = "sprites")]
use anyhow::{Context, Result};
#[cfg(feature = "sprites")]
use directories::ProjectDirs;
#[cfg(feature = "sprites")]
use image::{DynamicImage, GenericImageView, RgbaImage};
#[cfg(feature = "sprites")]
use reqwest::blocking::Client;
#[cfg(feature = "sprites")]
use std::collections::HashMap;
#[cfg(feature = "sprites")]
use std::path::{Path, PathBuf};

/// スプライト画像の取得・表示を管理するサービス
#[cfg(feature = "sprites")]
pub struct SpriteService {
    cache_dir: PathBuf,
    client: Client,
    base_url: String,
    id_map: HashMap<String, u32>,
}

#[cfg(feature = "sprites")]
impl SpriteService {
    /// 新しいSpriteServiceインスタンスを作成
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

    /// 英名からポケモンIDを取得
    pub fn get_pokemon_id(&self, english_name: &str) -> Option<u32> {
        self.id_map.get(english_name).copied()
    }

    /// ポケモンIDからローカルキャッシュのスプライトパスを取得
    pub fn get_sprite_path(&self, pokemon_id: u32) -> PathBuf {
        self.cache_dir.join(format!("{}.png", pokemon_id))
    }

    /// 後ろ姿のローカルキャッシュパス。前姿と衝突しないよう別名にする
    pub fn get_back_sprite_path(&self, pokemon_id: u32) -> PathBuf {
        self.cache_dir.join(format!("{}_back.png", pokemon_id))
    }

    /// 前姿と後ろ姿を横並びで表示。後ろ姿が取れなければ前姿だけ表示する
    pub fn display_sprite_for_pokemon(&self, english_name: &str) -> Result<()> {
        let Some(pokemon_id) = self.get_pokemon_id(english_name) else {
            return Ok(());
        };
        // 前姿が取れないときは静かに何も出さない
        let Ok(front_path) = self.fetch_sprite(pokemon_id) else {
            return Ok(());
        };

        let back = self
            .fetch_back_sprite(pokemon_id)
            .ok()
            .and_then(|path| image::open(path).ok());
        let front = image::open(&front_path).ok();

        let (cols, rows) = crossterm::terminal::size().unwrap_or((80, 24));
        if let (Some(front), Some(back)) = (front, back) {
            let combined = compose_side_by_side(&front, &back);
            if fits_side_by_side(
                front.dimensions(),
                combined.dimensions(),
                u32::from(cols),
                u32::from(rows),
            ) {
                return self.print_image(&combined, &front_path);
            }
        }
        self.display_sprite(&front_path)
    }

    /// PokeAPIからスプライト画像をダウンロード
    pub fn fetch_sprite(&self, pokemon_id: u32) -> Result<PathBuf> {
        self.fetch_cached(
            self.get_sprite_path(pokemon_id),
            &format!("sprites/pokemon/{}.png", pokemon_id),
            pokemon_id,
        )
    }

    /// PokeAPIから後ろ姿をダウンロード
    pub fn fetch_back_sprite(&self, pokemon_id: u32) -> Result<PathBuf> {
        self.fetch_cached(
            self.get_back_sprite_path(pokemon_id),
            &format!("sprites/pokemon/back/{}.png", pokemon_id),
            pokemon_id,
        )
    }

    fn fetch_cached(
        &self,
        sprite_path: PathBuf,
        url_path: &str,
        pokemon_id: u32,
    ) -> Result<PathBuf> {
        if sprite_path.exists() {
            return Ok(sprite_path);
        }

        let url = format!("{}/{}", self.base_url, url_path);

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

    /// スプライト画像をターミナルに表示
    pub fn display_sprite(&self, sprite_path: &Path) -> Result<()> {
        let img = image::open(sprite_path)
            .with_context(|| format!("Failed to open sprite image: {}", sprite_path.display()))?;
        self.print_image(&img, sprite_path)
    }

    fn print_image(&self, img: &DynamicImage, sprite_path: &Path) -> Result<()> {
        let config = viuer::Config {
            transparent: true,
            absolute_offset: false,
            ..Default::default()
        };

        if let Err(e) = viuer::print(img, &config) {
            // viuer が使えない端末ではパスだけ案内する
            println!("🖼️  Sprite saved at: {}", sprite_path.display());
            println!("   (Terminal image display not available: {})", e);
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

/// 並べた画像を端末に収めても、前姿だけのときより縮まないか。
/// 縮むと荒くなるので、収まらない（半幅ペインなど）ときは前姿だけを出す。
/// viuer は画像を端末の桁数×行数（1行=縦2px換算）に収めるため、その倍率を比べる
#[cfg(feature = "sprites")]
fn fits_side_by_side(front: (u32, u32), combined: (u32, u32), cols: u32, rows: u32) -> bool {
    let scale = |(w, h): (u32, u32)| {
        let by_width = f64::from(cols) / f64::from(w);
        let by_height = f64::from(2 * rows) / f64::from(h);
        1.0_f64.min(by_width).min(by_height)
    };
    scale(combined) >= scale(front)
}

/// 前姿を左、後ろ姿を右に並べた1枚の画像にする。
/// 1枚にまとめるのは、viuer のカーソル制御に頼らず横並びを保つため
#[cfg(feature = "sprites")]
fn compose_side_by_side(front: &DynamicImage, back: &DynamicImage) -> DynamicImage {
    let (fw, fh) = front.dimensions();
    let (bw, bh) = back.dimensions();
    let mut canvas = RgbaImage::new(fw + bw, fh.max(bh));
    image::imageops::overlay(&mut canvas, &front.to_rgba8(), 0, 0);
    image::imageops::overlay(&mut canvas, &back.to_rgba8(), i64::from(fw), 0);
    DynamicImage::ImageRgba8(canvas)
}

#[cfg(test)]
#[cfg(feature = "sprites")]
mod tests {
    use super::*;
    use image::{Rgba, RgbaImage};
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
    fn test_back_sprite_path_is_separate_from_front() {
        let temp_dir = tempdir().unwrap();
        let service = SpriteService::for_test(temp_dir.path().to_path_buf(), HashMap::new());

        assert_eq!(
            service.get_back_sprite_path(25),
            temp_dir.path().join("25_back.png")
        );
        assert_ne!(
            service.get_back_sprite_path(25),
            service.get_sprite_path(25)
        );
    }

    #[test]
    fn test_fetch_back_sprite_downloads_from_back_dir_and_caches() {
        use httpmock::prelude::*;

        let server = MockServer::start();
        let mock = server.mock(|when, then| {
            when.method(GET).path("/sprites/pokemon/back/25.png");
            then.status(200).body(b"back_image");
        });

        let temp_dir = tempdir().unwrap();
        let service = SpriteService::with_base_url(
            temp_dir.path().to_path_buf(),
            Client::new(),
            server.url(""),
        );

        let path = service.fetch_back_sprite(25).unwrap();
        assert_eq!(path, temp_dir.path().join("25_back.png"));
        assert_eq!(fs::read(&path).unwrap(), b"back_image");
        mock.assert();
        // 2回目はキャッシュを使うので再ダウンロードしない
        service.fetch_back_sprite(25).unwrap();
        mock.assert_hits(1);
    }

    #[test]
    fn test_fetch_back_sprite_404_creates_no_file() {
        use httpmock::prelude::*;

        let server = MockServer::start();
        let _mock = server.mock(|when, then| {
            when.method(GET).path("/sprites/pokemon/back/10264.png");
            then.status(404);
        });

        let temp_dir = tempdir().unwrap();
        let service = SpriteService::with_base_url(
            temp_dir.path().to_path_buf(),
            Client::new(),
            server.url(""),
        );

        assert!(service.fetch_back_sprite(10264).is_err());
        assert!(!service.get_back_sprite_path(10264).exists());
    }

    #[test]
    fn test_compose_side_by_side_puts_front_left_and_back_right() {
        let front = DynamicImage::ImageRgba8(RgbaImage::from_pixel(4, 3, Rgba([255, 0, 0, 255])));
        let back = DynamicImage::ImageRgba8(RgbaImage::from_pixel(5, 3, Rgba([0, 0, 255, 255])));

        let combined = compose_side_by_side(&front, &back).to_rgba8();

        assert_eq!(combined.dimensions(), (9, 3));
        assert_eq!(combined.get_pixel(0, 0), &Rgba([255, 0, 0, 255]));
        assert_eq!(combined.get_pixel(4, 0), &Rgba([0, 0, 255, 255]));
    }

    #[test]
    fn test_side_by_side_fits_in_wide_terminal() {
        // 前姿の表示倍率(桁/px)を保てるなら並べる
        assert!(fits_side_by_side((96, 96), (192, 96), 200, 100));
    }

    #[test]
    fn test_side_by_side_does_not_fit_in_half_width_pane() {
        // 並べると端末幅に収めるため縮み、前姿だけのときより荒くなる
        assert!(!fits_side_by_side((96, 96), (192, 96), 100, 60));
    }

    #[test]
    fn test_side_by_side_when_height_is_the_limit() {
        // 高さで制限される端末では、並べても倍率が変わらない幅があれば並べる
        assert!(fits_side_by_side((96, 96), (192, 96), 100, 20));
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
