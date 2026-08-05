#[cfg(feature = "cries")]
use anyhow::{Context, Result};
#[cfg(feature = "cries")]
use directories::ProjectDirs;
#[cfg(feature = "cries")]
use reqwest::blocking::Client;
#[cfg(feature = "cries")]
use std::collections::HashMap;
#[cfg(feature = "cries")]
use std::path::{Path, PathBuf};

/// 鳴き声の取得・再生を管理するサービス
#[cfg(feature = "cries")]
pub struct CryService {
    cache_dir: PathBuf,
    client: Client,
    base_url: String,
    id_map: HashMap<String, u32>,
}

#[cfg(feature = "cries")]
impl CryService {
    /// 新しいCryServiceインスタンスを作成
    pub fn new() -> Result<Self> {
        use crate::data::DataLoader;

        let project_dirs = ProjectDirs::from("", "", "poke-lookup")
            .or_else(|| ProjectDirs::from("dev", "poke-lookup", "poke-lookup"))
            .context("Failed to determine project directories")?;

        let cache_dir = project_dirs.data_dir().join("cries");

        if !cache_dir.exists() {
            std::fs::create_dir_all(&cache_dir).with_context(|| {
                format!(
                    "Failed to create cry cache directory: {}",
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
            base_url: "https://raw.githubusercontent.com/PokeAPI/cries/main".to_string(),
            id_map,
        })
    }

    /// 英名からポケモンIDを取得
    pub fn get_pokemon_id(&self, english_name: &str) -> Option<u32> {
        self.id_map.get(english_name).copied()
    }

    /// ポケモンIDからローカルキャッシュの鳴き声パスを取得
    pub fn get_cry_path(&self, pokemon_id: u32) -> PathBuf {
        self.cache_dir.join(format!("{}.ogg", pokemon_id))
    }

    /// ポケモンの鳴き声を取得して再生
    pub fn play_cry_for_pokemon(&self, english_name: &str) -> Result<()> {
        if let Some(pokemon_id) = self.get_pokemon_id(english_name) {
            match self.fetch_cry(pokemon_id) {
                Ok(cry_path) => {
                    // 再生失敗（音声デバイスなし等）は機能の主目的ではないので静かに無視
                    let _ = self.play_cry(&cry_path);
                }
                Err(_) => {
                    // 静かに失敗
                }
            }
        }
        Ok(())
    }

    /// PokeAPIのcriesリポジトリから鳴き声をダウンロード
    pub fn fetch_cry(&self, pokemon_id: u32) -> Result<PathBuf> {
        let cry_path = self.get_cry_path(pokemon_id);

        if cry_path.exists() {
            return Ok(cry_path);
        }

        let url = format!("{}/cries/pokemon/latest/{}.ogg", self.base_url, pokemon_id);

        let response = self
            .client
            .get(&url)
            .send()
            .with_context(|| format!("Failed to fetch cry for Pokemon ID {}", pokemon_id))?;

        if !response.status().is_success() {
            return Err(anyhow::anyhow!(
                "Failed to download cry for Pokemon ID {}: HTTP {}",
                pokemon_id,
                response.status()
            ));
        }

        let content = response.bytes().context("Failed to read cry data")?;

        std::fs::write(&cry_path, content)
            .with_context(|| format!("Failed to save cry to {}", cry_path.display()))?;

        Ok(cry_path)
    }

    /// 鳴き声を再生（再生完了まで待つ）
    pub fn play_cry(&self, cry_path: &Path) -> Result<()> {
        use std::fs::File;
        use std::io::BufReader;

        let mut handle = rodio::DeviceSinkBuilder::open_default_sink()
            .context("Failed to open default audio device")?;
        // drop 時の警告ログが CLI 出力に混ざるのを防ぐ
        handle.log_on_drop(false);

        let file = BufReader::new(
            File::open(cry_path)
                .with_context(|| format!("Failed to open cry file: {}", cry_path.display()))?,
        );

        // 拡張子は .ogg だが実体は MP3。Decoder は中身で判定するのでそのまま渡せる
        let player = rodio::play(handle.mixer(), file)
            .with_context(|| format!("Failed to play cry: {}", cry_path.display()))?;

        // 待たずに抜けると handle が drop されて音が途中で切れる
        player.sleep_until_end();

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

// 再生そのものは音声デバイスを必要とするため CI では検証できない。
// ここではダウンロード・キャッシュ・ID解決のみをテストする。
#[cfg(test)]
#[cfg(feature = "cries")]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn test_for_test_creates_service() {
        let temp_dir = tempdir().unwrap();
        let id_map = HashMap::new();

        let service = CryService::for_test(temp_dir.path().to_path_buf(), id_map);
        assert_eq!(service.base_url, "test://mock");
    }

    #[test]
    fn test_cry_path() {
        let temp_dir = tempdir().unwrap();
        let service = CryService::for_test(temp_dir.path().to_path_buf(), HashMap::new());

        assert_eq!(service.get_cry_path(25), temp_dir.path().join("25.ogg"));
        assert_eq!(service.get_cry_path(1), temp_dir.path().join("1.ogg"));
    }

    #[test]
    fn test_get_pokemon_id() {
        let temp_dir = tempdir().unwrap();
        let mut id_map = HashMap::new();
        id_map.insert("Pikachu".to_string(), 25);
        id_map.insert("Bulbasaur".to_string(), 1);

        let service = CryService::for_test(temp_dir.path().to_path_buf(), id_map);

        assert_eq!(service.get_pokemon_id("Pikachu"), Some(25));
        assert_eq!(service.get_pokemon_id("Bulbasaur"), Some(1));
        assert_eq!(service.get_pokemon_id("Unknown"), None);
    }

    #[test]
    fn test_fetch_cry_cached() {
        let temp_dir = tempdir().unwrap();
        let service = CryService::with_base_url(
            temp_dir.path().to_path_buf(),
            Client::new(),
            "http://dummy.example.com".to_string(),
        );

        let cry_path = service.get_cry_path(25);
        fs::write(&cry_path, b"cached_audio").unwrap();

        // キャッシュがあればダウンロードせずにそのパスを返す
        let result = service.fetch_cry(25);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), cry_path);

        let content = fs::read(&cry_path).unwrap();
        assert_eq!(content, b"cached_audio");
    }

    #[test]
    fn test_fetch_cry_download_success() {
        use httpmock::prelude::*;

        let server = MockServer::start();

        let mock_audio = b"ID3\x04\x00\x00\x00\x00\x00\x00dummy audio data".to_vec();

        let mock = server.mock(|when, then| {
            when.method(GET).path("/cries/pokemon/latest/25.ogg");
            then.status(200)
                .header("content-type", "audio/ogg")
                .body(&mock_audio);
        });

        let temp_dir = tempdir().unwrap();
        let client = Client::builder()
            .user_agent("poke-lookup/0.1.0")
            .build()
            .unwrap();

        let service =
            CryService::with_base_url(temp_dir.path().to_path_buf(), client, server.url(""));

        let result = service.fetch_cry(25);
        assert!(result.is_ok());

        let cry_path = result.unwrap();
        assert!(cry_path.exists());
        assert_eq!(fs::read(&cry_path).unwrap(), mock_audio);

        mock.assert();
    }

    #[test]
    fn test_fetch_cry_download_failure() {
        use httpmock::prelude::*;

        let server = MockServer::start();

        let _mock = server.mock(|when, then| {
            when.method(GET).path("/cries/pokemon/latest/9999.ogg");
            then.status(404)
                .header("content-type", "text/html")
                .body("Not Found");
        });

        let temp_dir = tempdir().unwrap();
        let service =
            CryService::with_base_url(temp_dir.path().to_path_buf(), Client::new(), server.url(""));

        let result = service.fetch_cry(9999);
        assert!(result.is_err());

        // 失敗時に壊れたファイルを残さない
        assert!(!service.get_cry_path(9999).exists());
    }
}
