use crate::models::NameDictionary;
use anyhow::{Context, Result};
use directories::ProjectDirs;
use std::fs;
use std::path::{Path, PathBuf};

/// データアクセス層
pub struct DataLoader {
    data_path: PathBuf,
}

impl DataLoader {
    /// 新しいDataLoaderインスタンスを作成
    /// XDGディレクトリ規約に従ってデフォルトパスを設定
    pub fn new() -> Result<Self> {
        let data_path = Self::get_default_data_path()?;
        Ok(Self { data_path })
    }

    /// 指定されたパスでDataLoaderインスタンスを作成
    pub fn with_path<P: Into<PathBuf>>(path: P) -> Self {
        Self {
            data_path: path.into(),
        }
    }

    /// XDG Data Directoryに基づくデフォルトパスを取得
    fn get_default_data_path() -> Result<PathBuf> {
        // ProjectDirs::from() が失敗する可能性があるため、代替案も用意
        let project_dirs = ProjectDirs::from("", "", "poke-lookup")
            .or_else(|| ProjectDirs::from("dev", "poke-lookup", "poke-lookup"))
            .context("Failed to determine project directories")?;

        let data_dir = project_dirs.data_dir();
        Ok(data_dir.join("names.json"))
    }

    /// names.jsonを読み込んでNameDictionaryを返す
    pub fn load_dictionary(&self) -> Result<NameDictionary> {
        // ファイルが存在しない場合のエラーメッセージを改善
        if !self.data_path.exists() {
            return Err(anyhow::anyhow!(
                "Data file not found: {}. Run 'poke-lookup update' to download the data file.",
                self.data_path.display()
            ));
        }

        let content = fs::read_to_string(&self.data_path)
            .with_context(|| format!("Failed to read file: {}", self.data_path.display()))?;

        let dictionary: NameDictionary = serde_json::from_str(&content)
            .with_context(|| format!("Failed to parse JSON: {}", self.data_path.display()))?;

        // データの検証
        dictionary
            .validate()
            .map_err(|e| anyhow::anyhow!("Data validation failed: {}", e))?;

        Ok(dictionary)
    }


    /// データファイルのパスを取得
    #[allow(dead_code)] // updateコマンドで使用予定
    pub fn data_path(&self) -> &Path {
        &self.data_path
    }

    /// データディレクトリの存在を保証（なければ作成）
    #[allow(dead_code)] // updateコマンドで使用予定
    pub fn ensure_data_dir(&self) -> Result<()> {
        if let Some(parent) = self.data_path.parent() {
            fs::create_dir_all(parent).with_context(|| {
                format!("Failed to create data directory: {}", parent.display())
            })?;
        }
        Ok(())
    }

    /// データファイルが存在するかチェック
    #[allow(dead_code)] // updateコマンドで使用予定
    pub fn data_exists(&self) -> bool {
        self.data_path.exists()
    }
}

impl Default for DataLoader {
    fn default() -> Self {
        // new()がエラーになる場合はテンポラリディレクトリを使用
        Self::new().unwrap_or_else(|_| {
            let temp_path = std::env::temp_dir().join("poke-lookup").join("names.json");
            Self::with_path(temp_path)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{NameDictionary, NameEntry};
    use chrono::Utc;
    use std::fs;
    use tempfile::tempdir;

    fn create_test_data() -> NameDictionary {
        NameDictionary {
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
        }
    }

    #[test]
    fn test_with_path() {
        let path = PathBuf::from("/tmp/test.json");
        let loader = DataLoader::with_path(&path);
        assert_eq!(loader.data_path(), path);
    }

    #[test]
    fn test_load_dictionary_file_not_found() {
        let temp_dir = tempdir().unwrap();
        let non_existent_path = temp_dir.path().join("non_existent.json");
        let loader = DataLoader::with_path(non_existent_path);

        let result = loader.load_dictionary();
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Data file not found")
        );
    }

    #[test]
    fn test_load_dictionary_success() {
        let temp_dir = tempdir().unwrap();
        let test_file = temp_dir.path().join("names.json");

        // テストデータを作成してファイルに保存
        let test_data = create_test_data();
        let json_content = serde_json::to_string(&test_data).unwrap();
        fs::write(&test_file, json_content).unwrap();

        let loader = DataLoader::with_path(&test_file);
        let result = loader.load_dictionary().unwrap();

        assert_eq!(result.schema_version, 1);
        assert_eq!(result.count, 2);
        assert_eq!(result.entries.len(), 2);
    }

    #[test]
    fn test_load_search_map() {
        let temp_dir = tempdir().unwrap();
        let test_file = temp_dir.path().join("names.json");

        let test_data = create_test_data();
        let json_content = serde_json::to_string(&test_data).unwrap();
        fs::write(&test_file, json_content).unwrap();

        let loader = DataLoader::with_path(&test_file);
        let dictionary = loader.load_dictionary().unwrap();
        let search_map = dictionary.to_hashmap();

        assert_eq!(search_map.get("ピカチュウ"), Some(&"Pikachu".to_string()));
        assert_eq!(search_map.get("フシギダネ"), Some(&"Bulbasaur".to_string()));
        assert_eq!(search_map.len(), 2);
    }

    #[test]
    fn test_ensure_data_dir() {
        let temp_dir = tempdir().unwrap();
        let test_path = temp_dir
            .path()
            .join("nested")
            .join("dir")
            .join("names.json");
        let loader = DataLoader::with_path(&test_path);

        // ディレクトリ作成が成功することを確認
        assert!(loader.ensure_data_dir().is_ok());
        assert!(test_path.parent().unwrap().exists());
    }

    #[test]
    #[cfg(unix)] // Unix系OSでのみ実行
    fn test_ensure_data_dir_permission_error() {
        use std::os::unix::fs::PermissionsExt;

        let temp_dir = tempdir().unwrap();
        let protected_dir = temp_dir.path().join("protected");
        fs::create_dir(&protected_dir).unwrap();

        // 書き込み権限を削除
        let mut perms = fs::metadata(&protected_dir).unwrap().permissions();
        perms.set_mode(0o555); // 読み取り専用
        fs::set_permissions(&protected_dir, perms).unwrap();

        let test_path = protected_dir.join("subdir").join("names.json");
        let loader = DataLoader::with_path(&test_path);

        // エラーが発生することを確認
        let result = loader.ensure_data_dir();
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Failed to create data directory")
        );

        // クリーンアップ：権限を戻す
        let mut perms = fs::metadata(&protected_dir).unwrap().permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&protected_dir, perms).unwrap();
    }

    #[test]
    fn test_data_exists() {
        let temp_dir = tempdir().unwrap();
        let existing_file = temp_dir.path().join("existing.json");
        let non_existing_file = temp_dir.path().join("non_existing.json");

        fs::write(&existing_file, "{}").unwrap();

        let loader_existing = DataLoader::with_path(&existing_file);
        let loader_non_existing = DataLoader::with_path(&non_existing_file);

        assert!(loader_existing.data_exists());
        assert!(!loader_non_existing.data_exists());
    }

    #[test]
    fn test_get_default_data_path() {
        let result = DataLoader::get_default_data_path();
        // XDGディレクトリが利用可能な場合のみテスト
        if result.is_ok() {
            let path = result.unwrap();
            assert!(path.to_string_lossy().contains("poke-lookup"));
            assert!(path.file_name().unwrap() == "names.json");
        }
    }
}
