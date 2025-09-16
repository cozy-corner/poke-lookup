use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// names.jsonのルート構造
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NameDictionary {
    /// スキーマバージョン
    pub schema_version: u32,
    /// 生成日時
    pub generated_at: DateTime<Utc>,
    /// エントリ数
    pub count: usize,
    /// ポケモン名のエントリ
    pub entries: Vec<NameEntry>,
}

/// 個別のポケモン名エントリ
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct NameEntry {
    /// 日本語名（カタカナ）
    pub ja: String,
    /// 英名
    pub en: String,
}

impl NameDictionary {
    /// エントリをHashMapに変換（高速検索用）
    pub fn to_hashmap(&self) -> HashMap<String, String> {
        self.entries
            .iter()
            .map(|entry| (entry.ja.clone(), entry.en.clone()))
            .collect()
    }

    /// スキーマバージョンの検証
    pub fn validate_schema(&self) -> Result<(), String> {
        const EXPECTED_VERSION: u32 = 1;
        if self.schema_version != EXPECTED_VERSION {
            return Err(format!(
                "Schema version mismatch: expected {}, got {}",
                EXPECTED_VERSION, self.schema_version
            ));
        }
        Ok(())
    }

    /// エントリ数の検証
    pub fn validate_count(&self) -> Result<(), String> {
        if self.entries.len() != self.count {
            return Err(format!(
                "Entry count mismatch: expected {}, got {}",
                self.count,
                self.entries.len()
            ));
        }
        Ok(())
    }

    /// データ全体の検証
    pub fn validate(&self) -> Result<(), String> {
        self.validate_schema()?;
        self.validate_count()?;
        self.validate_entries()?;
        Ok(())
    }

    /// エントリの妥当性検証
    pub fn validate_entries(&self) -> Result<(), String> {
        // 空のエントリチェック
        for (i, entry) in self.entries.iter().enumerate() {
            if entry.ja.is_empty() {
                return Err(format!("Empty Japanese name at entry {}", i));
            }
            if entry.en.is_empty() {
                return Err(format!("Empty English name at entry {}", i));
            }
        }

        // 最小/最大エントリ数チェック
        if self.count < 1 {
            return Err("Entry count must be at least 1".to_string());
        }

        if self.count > 10000 {
            return Err(format!(
                "Entry count {} exceeds maximum limit of 10000",
                self.count
            ));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    #[test]
    fn test_deserialize_name_dictionary() {
        let json = r#"{
            "schema_version": 1,
            "generated_at": "2025-01-01T00:00:00Z",
            "count": 2,
            "entries": [
                {"ja": "ピカチュウ", "en": "Pikachu"},
                {"ja": "フシギダネ", "en": "Bulbasaur"}
            ]
        }"#;

        let dict: NameDictionary = serde_json::from_str(json).unwrap();
        assert_eq!(dict.schema_version, 1);
        assert_eq!(dict.count, 2);
        assert_eq!(dict.entries.len(), 2);
        assert_eq!(dict.entries[0].ja, "ピカチュウ");
        assert_eq!(dict.entries[0].en, "Pikachu");
    }

    #[test]
    fn test_to_hashmap() {
        let dict = NameDictionary {
            schema_version: 1,
            generated_at: Utc.with_ymd_and_hms(2025, 1, 1, 0, 0, 0).unwrap(),
            count: 2,
            entries: vec![
                NameEntry {
                    ja: "ピカチュウ".to_string(),
                    en: "Pikachu".to_string(),
                },
                NameEntry {
                    ja: "フシギダネ".to_string(),
                    en: "Bulbasaur".to_string(),
                },
            ],
        };

        let map = dict.to_hashmap();
        assert_eq!(map.get("ピカチュウ"), Some(&"Pikachu".to_string()));
        assert_eq!(map.get("フシギダネ"), Some(&"Bulbasaur".to_string()));
    }

    #[test]
    fn test_validate_schema() {
        let mut dict = NameDictionary {
            schema_version: 1,
            generated_at: Utc::now(),
            count: 0,
            entries: vec![],
        };

        assert!(dict.validate_schema().is_ok());

        dict.schema_version = 2;
        assert!(dict.validate_schema().is_err());
    }

    #[test]
    fn test_validate_count() {
        let dict = NameDictionary {
            schema_version: 1,
            generated_at: Utc::now(),
            count: 2,
            entries: vec![
                NameEntry {
                    ja: "ピカチュウ".to_string(),
                    en: "Pikachu".to_string(),
                },
                NameEntry {
                    ja: "フシギダネ".to_string(),
                    en: "Bulbasaur".to_string(),
                },
            ],
        };

        assert!(dict.validate_count().is_ok());

        let dict_invalid = NameDictionary {
            schema_version: 1,
            generated_at: Utc::now(),
            count: 3,
            entries: vec![NameEntry {
                ja: "ピカチュウ".to_string(),
                en: "Pikachu".to_string(),
            }],
        };

        assert!(dict_invalid.validate_count().is_err());
    }

    #[test]
    fn test_validate() {
        let dict = NameDictionary {
            schema_version: 1,
            generated_at: Utc::now(),
            count: 1,
            entries: vec![NameEntry {
                ja: "ピカチュウ".to_string(),
                en: "Pikachu".to_string(),
            }],
        };

        assert!(dict.validate().is_ok());
    }

    #[test]
    fn test_validate_entries_empty_names() {
        let dict = NameDictionary {
            schema_version: 1,
            generated_at: Utc::now(),
            count: 1,
            entries: vec![NameEntry {
                ja: "".to_string(),
                en: "Pikachu".to_string(),
            }],
        };

        assert!(dict.validate_entries().is_err());
    }

    #[test]
    fn test_validate_entries_zero_count() {
        let dict = NameDictionary {
            schema_version: 1,
            generated_at: Utc::now(),
            count: 0,
            entries: vec![],
        };

        assert!(dict.validate_entries().is_err());
        assert!(
            dict.validate_entries()
                .unwrap_err()
                .contains("must be at least 1")
        );
    }

    #[test]
    fn test_validate_entries_exceed_limit() {
        let dict = NameDictionary {
            schema_version: 1,
            generated_at: Utc::now(),
            count: 15000,
            entries: vec![],
        };

        assert!(dict.validate_entries().is_err());
        assert!(
            dict.validate_entries()
                .unwrap_err()
                .contains("exceeds maximum limit")
        );
    }
}
