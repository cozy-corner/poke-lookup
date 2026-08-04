use crate::search::SearchService;
#[cfg(feature = "sprites")]
use crate::sprite::SpriteService;
use anyhow::{Context, Result};
#[cfg(feature = "sprites")]
use crossterm::{
    event::{self, Event, KeyCode, KeyEvent},
    terminal::{disable_raw_mode, enable_raw_mode},
};
use skim::prelude::*;
#[cfg(feature = "sprites")]
use std::io::{self, Write};
use std::sync::Arc;

/// インタラクティブ選択のためのアイテム
#[derive(Debug, Clone)]
struct PokemonItem {
    japanese: String,
    english: String,
    /// 一覧に表示する文字列
    display: String,
    /// skim のマッチ対象。display にローマ字を足したもので、ローマ字は表示されない
    match_text: String,
}

impl SkimItem for PokemonItem {
    fn text(&self) -> std::borrow::Cow<'_, str> {
        self.match_text.as_str().into()
    }

    fn display<'a>(&'a self, context: DisplayContext<'a>) -> AnsiString<'a> {
        // ハイライト位置は match_text 上の位置なので、可視部からはみ出した分を捨てる
        let visible = self.display.chars().count() as u32;
        let fragments = match context.matches {
            Matches::CharIndices(indices) => indices
                .iter()
                .map(|&i| i as u32)
                .filter(|&i| i < visible)
                .map(|i| (context.highlight_attr, (i, i + 1)))
                .collect(),
            Matches::CharRange(start, end) => {
                let (start, end) = (start as u32, (end as u32).min(visible));
                if start < end {
                    vec![(context.highlight_attr, (start, end))]
                } else {
                    vec![]
                }
            }
            Matches::ByteRange(start, end) => {
                let start = context.text[..start].chars().count() as u32;
                let end = (context.text[..end].chars().count() as u32).min(visible);
                if start < end {
                    vec![(context.highlight_attr, (start, end))]
                } else {
                    vec![]
                }
            }
            Matches::None => vec![],
        };

        AnsiString::new_str(&self.display, fragments)
    }

    fn output(&self) -> std::borrow::Cow<'_, str> {
        self.english.as_str().into()
    }

    fn preview(&self, _context: PreviewContext) -> ItemPreview {
        ItemPreview::Text(format!("日本語: {}\n英語: {}", self.japanese, self.english))
    }
}

/// インタラクティブ選択機能
pub struct InteractiveSelector {
    search_service: SearchService,
    #[cfg(feature = "sprites")]
    sprite_service: Option<SpriteService>,
}

impl InteractiveSelector {
    /// 検索サービスからセレクターを作成
    pub fn new(search_service: SearchService) -> Self {
        #[cfg(feature = "sprites")]
        let sprite_service = SpriteService::new().ok();

        Self {
            search_service,
            #[cfg(feature = "sprites")]
            sprite_service,
        }
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
                self.run_skim_selection(&partial_matches, query)
            }
        }
    }

    /// 全候補からインタラクティブ選択（空クエリ時）
    #[allow(dead_code)] // CLIインターフェースで使用予定
    pub fn select_from_all(&self) -> Result<Option<String>> {
        let all_entries = self.search_service.all_entries();
        self.run_skim_selection(&all_entries, "")
    }

    /// skimを使用したインタラクティブ選択
    fn run_skim_selection(
        &self,
        candidates: &[(&str, &str)],
        initial_query: &str,
    ) -> Result<Option<String>> {
        // skim用のアイテムを作成
        let items: Vec<Arc<dyn SkimItem>> = candidates
            .iter()
            .map(|(ja, en)| {
                let display = format!("{} → {}", ja, en);
                let romaji = crate::romaji::variants(ja).join(" ");
                Arc::new(PokemonItem {
                    japanese: ja.to_string(),
                    english: en.to_string(),
                    match_text: format!("{} {}", display, romaji),
                    display,
                }) as Arc<dyn SkimItem>
            })
            .collect();

        // skimオプションを設定
        let options = SkimOptionsBuilder::default()
            .height(Some("40%"))
            // tuikit の終了処理は実行時の状態ではなくこのオプションで分岐する。
            // false のままだと、インラインモードで代替画面に入っていないのに
            // quit_alternate_screen だけを出すため、描画が消えずカーソルも戻らず、
            // 直後のスプライトが選択UIに重なる（issue #12）。
            .no_clear_start(true)
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
            let english_name = item.output().to_string();

            // スプライト表示とナビゲーション処理
            #[cfg(feature = "sprites")]
            if let Some(ref sprite_service) = self.sprite_service {
                if let Some(final_selection) = self.show_sprite_with_navigation(
                    &english_name,
                    sprite_service,
                    candidates,
                    initial_query,
                )? {
                    return Ok(Some(final_selection));
                } else {
                    // ESCが押されたら再選択のためにループに戻る
                    return self.run_skim_selection(candidates, initial_query);
                }
            }

            return Ok(Some(english_name));
        }

        Ok(None)
    }

    /// スプライトを表示して、ESC/ENTERでナビゲーション
    #[cfg(feature = "sprites")]
    fn show_sprite_with_navigation(
        &self,
        english_name: &str,
        sprite_service: &SpriteService,
        _candidates: &[(&str, &str)],
        _initial_query: &str,
    ) -> Result<Option<String>> {
        // スプライトを表示
        sprite_service.display_sprite_for_pokemon(english_name)?;

        // ナビゲーション指示を表示
        println!("\n📌 {} が選択されました", english_name);
        println!("   [Enter] 確定  [ESC] 再選択");
        io::stdout().flush()?;

        // raw modeを有効化してキー入力を待つ
        enable_raw_mode()?;

        let result = loop {
            if let Event::Key(KeyEvent { code, .. }) = event::read()? {
                match code {
                    KeyCode::Enter => {
                        disable_raw_mode()?;
                        break Some(english_name.to_string());
                    }
                    KeyCode::Esc => {
                        disable_raw_mode()?;
                        println!("\n🔄 再選択します...");
                        break None;
                    }
                    _ => {}
                }
            }
        };

        Ok(result)
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

    fn create_test_item() -> PokemonItem {
        let display = "フシギダネ → Bulbasaur".to_string();
        PokemonItem {
            japanese: "フシギダネ".to_string(),
            english: "Bulbasaur".to_string(),
            match_text: format!("{} fushigidane husigidane", display),
            display,
        }
    }

    #[test]
    fn test_text_contains_romaji() {
        let item = create_test_item();
        let text = item.text();
        assert!(text.contains("フシギダネ"));
        assert!(text.contains("Bulbasaur"));
        assert!(text.contains("fushigidane"));
        assert!(text.contains("husigidane"));
    }

    #[test]
    fn test_display_hides_romaji() {
        let item = create_test_item();
        let context = DisplayContext {
            text: &item.match_text,
            score: 0,
            matches: Matches::None,
            container_width: 80,
            highlight_attr: Default::default(),
        };

        assert_eq!(item.display(context).stripped(), "フシギダネ → Bulbasaur");
    }

    #[test]
    fn test_display_drops_highlight_outside_visible_part() {
        let item = create_test_item();
        // 隠しローマ字の位置だけにマッチした場合、可視部にハイライトは付かない。
        // AnsiString::new_str はフラグメントが1個かつ属性がデフォルトだと
        // 「属性なし」に潰すため、index は2個以上渡さないと has_attrs() が
        // クリップ処理をしなくても false のままになり、テストが空回りする。
        let hidden = item.display.chars().count() + 2;
        let context = DisplayContext {
            text: &item.match_text,
            score: 0,
            matches: Matches::CharIndices(&[hidden, hidden + 1]),
            container_width: 80,
            highlight_attr: Default::default(),
        };

        let rendered = item.display(context);
        assert_eq!(rendered.stripped(), "フシギダネ → Bulbasaur");
        assert!(!rendered.has_attrs());
    }

    #[test]
    fn test_display_keeps_highlight_inside_visible_part() {
        let item = create_test_item();
        let context = DisplayContext {
            text: &item.match_text,
            score: 0,
            matches: Matches::CharIndices(&[0, 1]),
            container_width: 80,
            highlight_attr: Default::default(),
        };

        assert!(item.display(context).has_attrs());
    }

    #[test]
    fn test_display_highlight_clip_boundary() {
        let item = create_test_item();
        let visible = item.display.chars().count();

        // 境界の1つ内側（visible - 1）は可視部として残る
        let context = DisplayContext {
            text: &item.match_text,
            score: 0,
            matches: Matches::CharIndices(&[visible - 1, visible - 2]),
            container_width: 80,
            highlight_attr: Default::default(),
        };
        assert!(item.display(context).has_attrs());

        // 境界ちょうど（visible）は隠しローマ字部として落ちる
        let context = DisplayContext {
            text: &item.match_text,
            score: 0,
            matches: Matches::CharIndices(&[visible, visible + 1]),
            container_width: 80,
            highlight_attr: Default::default(),
        };
        assert!(!item.display(context).has_attrs());
    }

    #[test]
    fn test_output_returns_english_name() {
        // 確定時の返り値は表示文字列ではなく英名そのもの
        assert_eq!(create_test_item().output(), "Bulbasaur");
    }

    #[test]
    fn test_pokemon_item_preview() {
        let item = create_test_item();

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
            assert!(text.contains("日本語: フシギダネ"));
            assert!(text.contains("英語: Bulbasaur"));
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
