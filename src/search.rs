use crate::data::DataLoader;
use anyhow::{Context, Result};
use std::collections::HashMap;

/// 検索サービス
#[derive(Clone)]
pub struct SearchService {
    /// 検索用HashMap（日本語名 -> 英名）
    name_map: HashMap<String, String>,
    /// 日本語名 -> タイプの英語スラッグ配列（タイプトークン生成用）
    type_map: HashMap<String, Vec<String>>,
}

impl SearchService {
    /// DataLoaderから検索サービスを作成
    pub fn from_loader(loader: &DataLoader) -> Result<Self> {
        let dictionary = loader
            .load_dictionary()
            .context("Failed to load dictionary")?;

        let name_map = dictionary.to_hashmap();
        let type_map = dictionary.to_type_map();

        Ok(Self { name_map, type_map })
    }

    /// HashMapから直接検索サービスを作成（テスト用）
    #[allow(dead_code)]
    pub fn from_name_map(name_map: HashMap<String, String>) -> Self {
        Self {
            name_map,
            type_map: HashMap::new(),
        }
    }

    /// name_map と type_map を直接渡して作成（テスト用）
    #[cfg(test)]
    pub fn from_maps(
        name_map: HashMap<String, String>,
        type_map: HashMap<String, Vec<String>>,
    ) -> Self {
        Self { name_map, type_map }
    }

    /// 日本語名から skim 用のタイプトークン列を作る。
    /// 各 slug を「日本語名 漢字表記 slug」に展開して半角空白区切りで並べる（例: "ほのお 炎 fire"）。
    /// 漢字表記は IME による変換後の入力（でんき→電気）を拾うためのエイリアス。
    /// 未知 slug は slug のみ。types が無ければ空文字。
    ///
    /// タイプが2つとも既知なら、日本語名を全角スペースで繋いだ組トークンを
    /// 表記の全組み合わせ・両順序で足す。
    /// skim は AND 区切りを半角スペースしか見ない（skim factory.rs の RE_AND）ため、
    /// 全角スペースで2タイプ指定しても引けるよう、haystack 側に仕込む。
    pub fn type_tokens(&self, japanese_name: &str) -> String {
        self.type_map
            .get(japanese_name)
            .map(|slugs| {
                // 既知 slug の表記候補（日本語名と、あれば漢字表記）
                let aliases: Vec<Vec<&str>> = slugs
                    .iter()
                    .filter_map(|slug| {
                        crate::pokemon_type::type_ja(slug).map(|ja| {
                            match crate::pokemon_type::type_kanji(slug) {
                                Some(kanji) => vec![ja, kanji],
                                None => vec![ja],
                            }
                        })
                    })
                    .collect();

                let mut tokens: Vec<String> = slugs
                    .iter()
                    .map(|slug| match crate::pokemon_type::type_ja(slug) {
                        Some(ja) => match crate::pokemon_type::type_kanji(slug) {
                            Some(kanji) => format!("{} {} {}", ja, kanji, slug),
                            None => format!("{} {}", ja, slug),
                        },
                        None => slug.clone(),
                    })
                    .collect();

                if aliases.len() == 2 {
                    for a in &aliases[0] {
                        for b in &aliases[1] {
                            tokens.push(format!("{}　{}", a, b));
                            tokens.push(format!("{}　{}", b, a));
                        }
                    }
                }

                tokens.join(" ")
            })
            .unwrap_or_default()
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

        SearchService {
            name_map,
            type_map: HashMap::new(),
        }
    }

    #[test]
    fn test_type_tokens() {
        let mut name_map = HashMap::new();
        name_map.insert("リザードン".to_string(), "Charizard".to_string());
        let mut type_map = HashMap::new();
        type_map.insert(
            "リザードン".to_string(),
            vec!["fire".to_string(), "flying".to_string()],
        );
        let service = SearchService::from_maps(name_map, type_map);

        // 個別トークンに続けて、全角スペースで繋いだタイプ2つの組を両順序で持つ
        assert_eq!(
            service.type_tokens("リザードン"),
            "ほのお 炎 fire ひこう 飛行 flying \
ほのお　ひこう ひこう　ほのお ほのお　飛行 飛行　ほのお \
炎　ひこう ひこう　炎 炎　飛行 飛行　炎"
        );
        // types 無し・未登録は空文字
        assert_eq!(service.type_tokens("ピカチュウ"), "");
    }

    #[test]
    fn test_type_tokens_fullwidth_pair_both_orders() {
        let mut type_map = HashMap::new();
        type_map.insert(
            "リザードン".to_string(),
            vec!["fire".to_string(), "flying".to_string()],
        );
        let service = SearchService::from_maps(HashMap::new(), type_map);

        let tokens = service.type_tokens("リザードン");
        // 全角スペース区切りは skim では AND にならないので、
        // haystack 側に両順序の組トークンを仕込んで引けるようにする
        assert!(tokens.contains("ほのお　ひこう"));
        assert!(tokens.contains("ひこう　ほのお"));
        // 漢字表記どうし・混在の組も引ける
        assert!(tokens.contains("炎　飛行"));
        assert!(tokens.contains("飛行　炎"));
        assert!(tokens.contains("ほのお　飛行"));
        assert!(tokens.contains("炎　ひこう"));
    }

    #[test]
    fn test_type_tokens_includes_kanji_alias() {
        let mut type_map = HashMap::new();
        type_map.insert("ピカチュウ".to_string(), vec!["electric".to_string()]);
        // 漢字表記を持たないタイプは日本語名のみ
        type_map.insert("ケーシィ".to_string(), vec!["psychic".to_string()]);
        let service = SearchService::from_maps(HashMap::new(), type_map);

        assert_eq!(service.type_tokens("ピカチュウ"), "でんき 電気 electric");
        assert_eq!(service.type_tokens("ケーシィ"), "エスパー psychic");
    }

    #[test]
    fn test_type_tokens_single_type_has_no_fullwidth_pair() {
        let mut type_map = HashMap::new();
        type_map.insert("ヒトカゲ".to_string(), vec!["fire".to_string()]);
        let service = SearchService::from_maps(HashMap::new(), type_map);

        let tokens = service.type_tokens("ヒトカゲ");
        assert_eq!(tokens, "ほのお 炎 fire");
        // 単タイプは全角スペースの組トークンを持たない
        assert!(!tokens.contains('　'));
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
            schema_version: 2,
            generated_at: Utc::now(),
            count: 2,
            entries: vec![
                NameEntry {
                    ja: "ピカチュウ".to_string(),
                    en: "Pikachu".to_string(),
                    id: None,
                    types: vec![],
                },
                NameEntry {
                    ja: "フシギダネ".to_string(),
                    en: "Bulbasaur".to_string(),
                    id: None,
                    types: vec![],
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
}
