use crate::search::SearchService;
use anyhow::{Context, Result};
use skim::prelude::*;
use std::sync::Arc;

/// インタラクティブ選択のためのアイテム
#[derive(Debug, Clone)]
struct PokemonItem {
    japanese: String,
    english: String,
    display: String,
}

impl SkimItem for PokemonItem {
    fn text(&self) -> std::borrow::Cow<'_, str> {
        self.display.as_str().into()
    }

    fn preview(&self, _context: PreviewContext) -> ItemPreview {
        ItemPreview::Text(format!("日本語: {}\n英語: {}", self.japanese, self.english))
    }
}

/// インタラクティブ選択機能
pub struct InteractiveSelector {
    search_service: SearchService,
}

impl InteractiveSelector {
    /// 検索サービスからセレクターを作成
    pub fn new(search_service: SearchService) -> Self {
        Self { search_service }
    }

    /// インタラクティブ選択を開始
    /// 戻り値: Ok(Some(english_name)) - 選択成功
    ///         Ok(None) - ユーザーキャンセル
    ///         Err - エラー発生
    #[allow(dead_code)] // CLIインターフェースで使用予定
    pub fn select_interactive(&self, query: &str) -> Result<Option<String>> {
        // まず完全一致を試す
        if let Some(exact) = self.search_service.search_exact(query) {
            return Ok(Some(exact.to_string()));
        }

        // 部分一致で候補を取得
        let partial_matches = self.search_service.search_partial(query);

        match partial_matches.len() {
            0 => Ok(None), // 候補なし
            _ => {
                // 候補があればインタラクティブ選択（1件でも）
                self.run_skim_selection(partial_matches, query)
            }
        }
    }

    /// 全候補からインタラクティブ選択（空クエリ時）
    #[allow(dead_code)] // CLIインターフェースで使用予定
    pub fn select_from_all(&self) -> Result<Option<String>> {
        let all_entries = self.search_service.all_entries();
        self.run_skim_selection(all_entries, "")
    }

    /// skimを使用したインタラクティブ選択
    fn run_skim_selection(
        &self,
        candidates: Vec<(&str, &str)>,
        initial_query: &str,
    ) -> Result<Option<String>> {
        // skim用のアイテムを作成
        let items: Vec<Arc<dyn SkimItem>> = candidates
            .into_iter()
            .map(|(ja, en)| {
                Arc::new(PokemonItem {
                    japanese: ja.to_string(),
                    english: en.to_string(),
                    display: format!("{} → {}", ja, en),
                }) as Arc<dyn SkimItem>
            })
            .collect();

        // skimオプションを設定
        let options = SkimOptionsBuilder::default()
            .height(Some("40%"))
            .multi(false)
            .preview(Some(""))
            .preview_window(Some("down:3:wrap"))
            .query(Some(initial_query))
            .prompt(Some("ポケモンを選択: "))
            .bind(vec!["ctrl-n:down", "ctrl-p:up", "ctrl-j:down", "ctrl-k:up"])
            .build()
            .context("Failed to build skim options")?;

        // チャンネルを作成してアイテムを送信
        let (tx_item, rx_item): (SkimItemSender, SkimItemReceiver) = unbounded();

        for item in items {
            let _ = tx_item.send(item);
        }
        drop(tx_item); // 送信完了を示すため

        // skimを実行
        let selected_items = Skim::run_with(&options, Some(rx_item))
            .context("Failed to run interactive selection")?;

        // 結果を処理
        if selected_items.is_abort {
            return Ok(None); // ユーザーがキャンセル
        }

        if let Some(item) = selected_items.selected_items.first() {
            // 選択されたアイテムから英名を抽出
            let text = item.text();
            if text.contains(" → ") {
                // UTF-8文字境界を考慮して分割
                let parts: Vec<&str> = text.split(" → ").collect();
                if parts.len() == 2 {
                    return Ok(Some(parts[1].trim().to_string()));
                }
            }
        }

        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn create_test_selector() -> InteractiveSelector {
        let mut name_map = HashMap::new();
        name_map.insert("ピカチュウ".to_string(), "Pikachu".to_string());
        name_map.insert("フシギダネ".to_string(), "Bulbasaur".to_string());
        name_map.insert("フシギソウ".to_string(), "Ivysaur".to_string());
        name_map.insert("フシギバナ".to_string(), "Venusaur".to_string());
        name_map.insert("ヒトカゲ".to_string(), "Charmander".to_string());

        let search_service = SearchService::from_name_map(name_map);
        InteractiveSelector::new(search_service)
    }

    #[test]
    fn test_pokemon_item_text() {
        let item = PokemonItem {
            japanese: "ピカチュウ".to_string(),
            english: "Pikachu".to_string(),
            display: "ピカチュウ → Pikachu".to_string(),
        };

        assert_eq!(item.text(), "ピカチュウ → Pikachu");
    }

    #[test]
    fn test_pokemon_item_preview() {
        let item = PokemonItem {
            japanese: "ピカチュウ".to_string(),
            english: "Pikachu".to_string(),
            display: "ピカチュウ → Pikachu".to_string(),
        };

        let preview_context = PreviewContext {
            query: "",
            cmd_query: "",
            current_index: 0,
            current_selection: "",
            selected_indices: &[],
            selections: &[],
            height: 10,
            width: 50,
        };

        let preview = item.preview(preview_context);
        if let ItemPreview::Text(text) = preview {
            assert!(text.contains("日本語: ピカチュウ"));
            assert!(text.contains("英語: Pikachu"));
        } else {
            panic!("Expected text preview");
        }
    }

    #[test]
    fn test_select_interactive_exact_match() {
        let selector = create_test_selector();

        // 完全一致の場合、即座に結果を返す（skimを起動しない）
        // このテストは実際のskimなしで動作確認
        let search_service = &selector.search_service;
        let exact = search_service.search_exact("ピカチュウ");
        assert_eq!(exact, Some("Pikachu"));
    }

    #[test]
    fn test_select_interactive_single_partial() {
        let selector = create_test_selector();

        // 部分一致が1件の場合の動作確認
        let partial_matches = selector.search_service.search_partial("ピカ");
        assert_eq!(partial_matches.len(), 1);
        assert_eq!(partial_matches[0], ("ピカチュウ", "Pikachu"));
    }

    #[test]
    fn test_select_interactive_multiple_partial() {
        let selector = create_test_selector();

        // 部分一致が複数件の場合の候補確認
        let partial_matches = selector.search_service.search_partial("フシギ");
        assert_eq!(partial_matches.len(), 3);
        assert!(partial_matches.contains(&("フシギダネ", "Bulbasaur")));
        assert!(partial_matches.contains(&("フシギソウ", "Ivysaur")));
        assert!(partial_matches.contains(&("フシギバナ", "Venusaur")));
    }

    #[test]
    fn test_select_interactive_no_match() {
        let selector = create_test_selector();

        // マッチしない場合の動作確認
        let partial_matches = selector.search_service.search_partial("ミュウツー");
        assert_eq!(partial_matches.len(), 0);
    }
}
