use crate::data::DataLoader;
use anyhow::{Context, Result};
use std::collections::HashMap;

/// 検索サービス
#[derive(Clone)]
pub struct SearchService {
    /// 検索用HashMap（日本語名 -> 英名）
    name_map: HashMap<String, String>,
    /// 完全なエントリデータ（ID取得用）
    entries: Vec<crate::models::NameEntry>,
}

impl SearchService {
    /// DataLoaderから検索サービスを作成
    pub fn from_loader(loader: &DataLoader) -> Result<Self> {
        let dictionary = loader
            .load_dictionary()
            .context("Failed to load dictionary")?;

        let name_map = dictionary.to_hashmap();
        let entries = dictionary.entries;

        Ok(Self { name_map, entries })
    }

    /// HashMapから直接検索サービスを作成（テスト用）
    #[allow(dead_code)]
    pub fn from_name_map(name_map: HashMap<String, String>) -> Self {
        Self {
            name_map,
            entries: Vec::new(),
        }
    }

    /// 新しい検索サービスインスタンスを作成（デフォルトパス使用）
    #[allow(dead_code)] // updateコマンドで使用予定
    pub fn new() -> Result<Self> {
        let loader = DataLoader::new()?;
        Self::from_loader(&loader)
    }

    /// カスタムパスから検索サービスを作成
    #[allow(dead_code)] // CLIインターフェースで使用予定
    pub fn with_path<P: Into<std::path::PathBuf>>(path: P) -> Result<Self> {
        let loader = DataLoader::with_path(path);
        Self::from_loader(&loader)
    }

    /// 日本語名から英名を検索（完全一致）
    #[allow(dead_code)] // CLIインターフェースで使用予定
    pub fn search_exact(&self, japanese_name: &str) -> Option<&str> {
        self.name_map.get(japanese_name).map(|s| s.as_str())
    }

    /// 部分一致検索（前方一致、後方一致、部分一致）
    pub fn search_partial(&self, query: &str) -> Vec<(&str, &str)> {
        let query_lower = query.to_lowercase();

        self.name_map
            .iter()
            .filter(|(ja, _)| {
                let ja_lower = ja.to_lowercase();
                ja_lower.contains(&query_lower)
            })
            .map(|(ja, en)| (ja.as_str(), en.as_str()))
            .collect()
    }

    /// 検索可能な全エントリ数を取得
    #[allow(dead_code)] // 更新機能で使用予定
    pub fn entry_count(&self) -> usize {
        self.name_map.len()
    }

    /// 全てのエントリを取得（インタラクティブ選択用）
    pub fn all_entries(&self) -> Vec<(&str, &str)> {
        self.name_map
            .iter()
            .map(|(ja, en)| (ja.as_str(), en.as_str()))
            .collect()
    }

    /// 英名からポケモンIDを取得
    pub fn get_pokemon_id(&self, english_name: &str) -> Option<u32> {
        self.entries
            .iter()
            .find(|entry| entry.en == english_name)
            .and_then(|entry| entry.id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{NameDictionary, NameEntry};
    use chrono::Utc;
    use std::fs;
    use tempfile::tempdir;

    fn create_test_service() -> SearchService {
        let mut name_map = HashMap::new();
        name_map.insert("ピカチュウ".to_string(), "Pikachu".to_string());
        name_map.insert("フシギダネ".to_string(), "Bulbasaur".to_string());
        name_map.insert("フシギソウ".to_string(), "Ivysaur".to_string());
        name_map.insert("フシギバナ".to_string(), "Venusaur".to_string());
        name_map.insert("ヒトカゲ".to_string(), "Charmander".to_string());

        let entries = vec![
            NameEntry {
                ja: "ピカチュウ".to_string(),
                en: "Pikachu".to_string(),
                id: Some(25),
            },
            NameEntry {
                ja: "フシギダネ".to_string(),
                en: "Bulbasaur".to_string(),
                id: Some(1),
            },
            NameEntry {
                ja: "フシギソウ".to_string(),
                en: "Ivysaur".to_string(),
                id: Some(2),
            },
            NameEntry {
                ja: "フシギバナ".to_string(),
                en: "Venusaur".to_string(),
                id: Some(3),
            },
            NameEntry {
                ja: "ヒトカゲ".to_string(),
                en: "Charmander".to_string(),
                id: Some(4),
            },
        ];

        SearchService { name_map, entries }
    }

    #[test]
    fn test_search_exact_found() {
        let service = create_test_service();
        assert_eq!(service.search_exact("ピカチュウ"), Some("Pikachu"));
        assert_eq!(service.search_exact("フシギダネ"), Some("Bulbasaur"));
    }

    #[test]
    fn test_search_exact_not_found() {
        let service = create_test_service();
        assert_eq!(service.search_exact("ミュウツー"), None);
        assert_eq!(service.search_exact("ピカ"), None); // 部分一致はしない
    }

    #[test]
    fn test_entry_count() {
        let service = create_test_service();
        assert_eq!(service.entry_count(), 5);
    }

    #[test]
    fn test_all_entries() {
        let service = create_test_service();
        let entries = service.all_entries();
        assert_eq!(entries.len(), 5);
        assert!(entries.contains(&("ピカチュウ", "Pikachu")));
    }

    #[test]
    fn test_from_loader() {
        let temp_dir = tempdir().unwrap();
        let test_file = temp_dir.path().join("names.json");

        let test_data = NameDictionary {
            schema_version: 1,
            generated_at: Utc::now(),
            count: 2,
            entries: vec![
                NameEntry {
                    ja: "ピカチュウ".to_string(),
                    en: "Pikachu".to_string(),
                    id: None,
                },
                NameEntry {
                    ja: "フシギダネ".to_string(),
                    en: "Bulbasaur".to_string(),
                    id: None,
                },
            ],
        };

        let json_content = serde_json::to_string(&test_data).unwrap();
        fs::write(&test_file, json_content).unwrap();

        let loader = DataLoader::with_path(&test_file);
        let service = SearchService::from_loader(&loader).unwrap();

        assert_eq!(service.search_exact("ピカチュウ"), Some("Pikachu"));
        assert_eq!(service.entry_count(), 2);
    }

    #[test]
    fn test_get_pokemon_id() {
        let service = create_test_service();
        assert_eq!(service.get_pokemon_id("Pikachu"), Some(25));
        assert_eq!(service.get_pokemon_id("Bulbasaur"), Some(1));
        assert_eq!(service.get_pokemon_id("Unknown"), None);
    }
}
