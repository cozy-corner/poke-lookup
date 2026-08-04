# ローマ字絞り込み 実装計画

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** インタラクティブ選択（skim）で、カタカナのポケモン名をローマ字入力で絞り込めるようにする。

**Architecture:** 新規モジュール `src/romaji.rs` がカタカナ名からヘボン式・訓令式のローマ字を生成する。`src/interactive.rs` の `PokemonItem` はそのローマ字を `text()`（skim のマッチ対象）にだけ含め、`display()`（一覧表示）には含めない。表示・マッチ・選択結果の 3 つを別々のメソッドに分離する。

**Tech Stack:** Rust 2024 edition / skim 0.10 / 依存クレートの追加なし

**設計ドキュメント:** `docs/superpowers/specs/2026-08-04-romaji-filter-design.md`

## Global Constraints

- 作業ディレクトリは worktree `/Users/sasakitakashinanji/code/poke-lookup-worktrees/feature-romaji-filter`（ブランチ `feature/romaji-filter`）。
- 依存クレートを追加しない。`Cargo.toml` は変更しない。
- `names.json` のスキーマを変更しない。`schema_version` は 1 のまま。`src/models.rs` / `src/data.rs` / `src/update.rs` は変更しない。
- `src/search.rs` は変更しない。ローマ字マッチはインタラクティブ選択のみが対象で、CLI 引数（`poke-lookup fushigi`）は従来どおりカタカナのみ。
- コミット時に pre-commit フックが `cargo fmt --all -- --check` / `cargo clippy --all-targets --all-features -- -D warnings` / `cargo test` を実行する（`scripts/hooks/pre-commit`）。警告 1 つでコミットが失敗するため、未使用アイテムには `#[allow(dead_code)]` を付ける。
- `tuikit::Attr` は skim から再エクスポートされていない。型名を書かずに `context.highlight_attr` や `Default::default()` から型推論させること。`let fragments: Vec<(Attr, (u32, u32))>` のような注釈はコンパイルできない。
- コメントは「なぜ」を書く。コードを読めば分かることは書かない（`CLAUDE.md` の方針）。

---

### Task 1: romaji モジュールの土台（単カナ・ヘボン式/訓令式）

1 文字のカタカナだけを変換する最小の `romaji` モジュールを作る。拗音・促音・長音は Task 2 / Task 3 で足す。

**Files:**
- Create: `src/romaji.rs`
- Modify: `src/main.rs`（`mod romaji;` を追加）
- Test: `src/romaji.rs` の `#[cfg(test)] mod tests`

**Interfaces:**
- Consumes: なし
- Produces: `pub fn variants(katakana: &str) -> Vec<String>` — ヘボン式と訓令式のローマ字を返す。両者が一致する場合は 1 要素。Task 4 が `src/interactive.rs` から呼ぶ。

- [ ] **Step 1: `src/main.rs` にモジュール宣言を追加**

`src/main.rs` の先頭のモジュール宣言（1-7 行目）はアルファベット順に並んでいる。`mod models;` の次に追加する。

```rust
mod data;
mod interactive;
mod models;
mod romaji;
mod search;
#[cfg(feature = "sprites")]
mod sprite;
mod update;
```

- [ ] **Step 2: 失敗するテストを書く**

`src/romaji.rs` を新規作成し、テストだけを書く。

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_single_kana() {
        assert_eq!(variants("ヒトカゲ"), vec!["hitokage"]);
        assert_eq!(variants("ピカ"), vec!["pika"]);
        assert_eq!(variants("ン"), vec!["n"]);
    }

    #[test]
    fn test_hepburn_and_kunrei_differ() {
        assert_eq!(variants("フシギ"), vec!["fushigi", "husigi"]);
        assert_eq!(variants("ツ"), vec!["tsu", "tu"]);
        assert_eq!(variants("チ"), vec!["chi", "ti"]);
        assert_eq!(variants("ジ"), vec!["ji", "zi"]);
    }

    #[test]
    fn test_identical_styles_are_deduped() {
        // 両式で差が出ない名前は 1 件だけ返る
        assert_eq!(variants("カメ").len(), 1);
    }

    #[test]
    fn test_unknown_char_passes_through() {
        // 変換表にない文字は落とさずそのまま通す
        assert_eq!(variants("ポリゴン2"), vec!["porigon2"]);
    }
}
```

- [ ] **Step 3: テストが失敗することを確認**

Run: `cargo test romaji::`
Expected: コンパイルエラー（`cannot find function 'variants'`）

- [ ] **Step 4: 最小の実装を書く**

`src/romaji.rs` のテストモジュールの前に追加する。

```rust
//! カタカナ名を、skim のマッチ用ローマ字表記に変換する。
//!
//! skim のマッチはファジー（部分列）マッチなので、長音を省いた入力
//! （buusutaa に対する busuta）は生成しなくてもヒットする。一方
//! husigi は fushigi の部分列にならないため、ヘボン式と訓令式の
//! 両方を生成する。

/// ローマ字の表記方式
#[derive(Clone, Copy, PartialEq, Eq)]
enum Style {
    Hepburn,
    Kunrei,
}

/// (カタカナ, ヘボン式, 訓令式)
const SYLLABLES: &[(&str, &str, &str)] = &[
    ("ア", "a", "a"),
    ("イ", "i", "i"),
    ("ウ", "u", "u"),
    ("エ", "e", "e"),
    ("オ", "o", "o"),
    ("カ", "ka", "ka"),
    ("キ", "ki", "ki"),
    ("ク", "ku", "ku"),
    ("ケ", "ke", "ke"),
    ("コ", "ko", "ko"),
    ("サ", "sa", "sa"),
    ("シ", "shi", "si"),
    ("ス", "su", "su"),
    ("セ", "se", "se"),
    ("ソ", "so", "so"),
    ("タ", "ta", "ta"),
    ("チ", "chi", "ti"),
    ("ツ", "tsu", "tu"),
    ("テ", "te", "te"),
    ("ト", "to", "to"),
    ("ナ", "na", "na"),
    ("ニ", "ni", "ni"),
    ("ヌ", "nu", "nu"),
    ("ネ", "ne", "ne"),
    ("ノ", "no", "no"),
    ("ハ", "ha", "ha"),
    ("ヒ", "hi", "hi"),
    ("フ", "fu", "hu"),
    ("ヘ", "he", "he"),
    ("ホ", "ho", "ho"),
    ("マ", "ma", "ma"),
    ("ミ", "mi", "mi"),
    ("ム", "mu", "mu"),
    ("メ", "me", "me"),
    ("モ", "mo", "mo"),
    ("ヤ", "ya", "ya"),
    ("ユ", "yu", "yu"),
    ("ヨ", "yo", "yo"),
    ("ラ", "ra", "ra"),
    ("リ", "ri", "ri"),
    ("ル", "ru", "ru"),
    ("レ", "re", "re"),
    ("ロ", "ro", "ro"),
    ("ワ", "wa", "wa"),
    ("ヲ", "o", "o"),
    ("ン", "n", "n"),
    ("ガ", "ga", "ga"),
    ("ギ", "gi", "gi"),
    ("グ", "gu", "gu"),
    ("ゲ", "ge", "ge"),
    ("ゴ", "go", "go"),
    ("ザ", "za", "za"),
    ("ジ", "ji", "zi"),
    ("ズ", "zu", "zu"),
    ("ゼ", "ze", "ze"),
    ("ゾ", "zo", "zo"),
    ("ダ", "da", "da"),
    ("ヂ", "ji", "zi"),
    ("ヅ", "zu", "zu"),
    ("デ", "de", "de"),
    ("ド", "do", "do"),
    ("バ", "ba", "ba"),
    ("ビ", "bi", "bi"),
    ("ブ", "bu", "bu"),
    ("ベ", "be", "be"),
    ("ボ", "bo", "bo"),
    ("パ", "pa", "pa"),
    ("ピ", "pi", "pi"),
    ("プ", "pu", "pu"),
    ("ペ", "pe", "pe"),
    ("ポ", "po", "po"),
    ("ヴ", "vu", "vu"),
    // 拗音の一部として現れなかった小書きカナ
    ("ァ", "a", "a"),
    ("ィ", "i", "i"),
    ("ゥ", "u", "u"),
    ("ェ", "e", "e"),
    ("ォ", "o", "o"),
    ("ャ", "ya", "ya"),
    ("ュ", "yu", "yu"),
    ("ョ", "yo", "yo"),
];

fn lookup(kana: &str, style: Style) -> Option<&'static str> {
    SYLLABLES
        .iter()
        .find(|(k, _, _)| *k == kana)
        .map(|(_, hepburn, kunrei)| match style {
            Style::Hepburn => *hepburn,
            Style::Kunrei => *kunrei,
        })
}

fn to_romaji(katakana: &str, style: Style) -> String {
    let mut out = String::new();
    for c in katakana.chars() {
        match lookup(&c.to_string(), style) {
            Some(r) => out.push_str(r),
            // 変換表にない文字は落とさずそのまま通す
            None => out.push(c),
        }
    }
    out
}

/// カタカナ名から、マッチ用のローマ字表記を返す（ヘボン式・訓令式、重複除去済み）
#[allow(dead_code)] // Task 4 で interactive.rs から使用する
pub fn variants(katakana: &str) -> Vec<String> {
    let hepburn = to_romaji(katakana, Style::Hepburn);
    let kunrei = to_romaji(katakana, Style::Kunrei);

    if hepburn == kunrei {
        vec![hepburn]
    } else {
        vec![hepburn, kunrei]
    }
}
```

- [ ] **Step 5: テストが通ることを確認**

Run: `cargo test romaji::`
Expected: 4 tests passed

- [ ] **Step 6: コミット**

```bash
git add src/romaji.rs src/main.rs
git commit -m "feat: カタカナを単カナ単位でローマ字に変換する romaji モジュールを追加

Co-Authored-By: Claude Opus 5 <noreply@anthropic.com>"
```

---

### Task 2: 拗音・外来音（2 文字で 1 音）

キャ・シュ・チョ などの拗音と、ファ・ウィ・ティ などの外来音を変換できるようにする。これらは 2 文字で 1 音なので、単カナより先に引く。

**Files:**
- Modify: `src/romaji.rs`（`SYLLABLES` に 2 文字エントリを追加、`to_romaji` を書き換え）
- Test: `src/romaji.rs` の `#[cfg(test)] mod tests`

**Interfaces:**
- Consumes: Task 1 の `SYLLABLES` / `lookup` / `to_romaji` / `Style`
- Produces: シグネチャの変更なし。`variants` の挙動が拡張されるのみ。

- [ ] **Step 1: 失敗するテストを書く**

`src/romaji.rs` の `mod tests` に追加する。

```rust
    #[test]
    fn test_youon() {
        assert_eq!(variants("キャ"), vec!["kya"]);
        assert_eq!(variants("シュ"), vec!["shu", "syu"]);
        assert_eq!(variants("チョ"), vec!["cho", "tyo"]);
        assert_eq!(variants("ジャ"), vec!["ja", "zya"]);
        assert_eq!(variants("リュ"), vec!["ryu"]);
    }

    #[test]
    fn test_foreign_sounds() {
        assert_eq!(variants("ファ"), vec!["fa"]);
        assert_eq!(variants("ウィ"), vec!["wi"]);
        assert_eq!(variants("ディ"), vec!["di"]);
        assert_eq!(variants("ジェ"), vec!["je", "zye"]);
        assert_eq!(variants("チェ"), vec!["che", "tye"]);
    }

    #[test]
    fn test_youon_in_name() {
        assert_eq!(variants("ピカチュウ"), vec!["pikachuu", "pikatyuu"]);
        assert_eq!(variants("ウィンディ"), vec!["windi"]);
    }
```

- [ ] **Step 2: テストが失敗することを確認**

Run: `cargo test romaji::`
Expected: `test_youon` が FAIL（`variants("キャ")` が `["kiya"]` を返す。ャ が単カナとして `ya` に変換されるため）

- [ ] **Step 3: 2 文字エントリを `SYLLABLES` に追加**

`src/romaji.rs` の `SYLLABLES` の**先頭**（`("ア", "a", "a")` の前）に挿入する。`lookup` はキー全体の完全一致なので順序に依存しないが、2 文字エントリをまとめて先頭に置くと読みやすい。

```rust
    // 拗音（2 文字で 1 音）
    ("キャ", "kya", "kya"),
    ("キュ", "kyu", "kyu"),
    ("キョ", "kyo", "kyo"),
    ("シャ", "sha", "sya"),
    ("シュ", "shu", "syu"),
    ("ショ", "sho", "syo"),
    ("チャ", "cha", "tya"),
    ("チュ", "chu", "tyu"),
    ("チョ", "cho", "tyo"),
    ("ニャ", "nya", "nya"),
    ("ニュ", "nyu", "nyu"),
    ("ニョ", "nyo", "nyo"),
    ("ヒャ", "hya", "hya"),
    ("ヒュ", "hyu", "hyu"),
    ("ヒョ", "hyo", "hyo"),
    ("ミャ", "mya", "mya"),
    ("ミュ", "myu", "myu"),
    ("ミョ", "myo", "myo"),
    ("リャ", "rya", "rya"),
    ("リュ", "ryu", "ryu"),
    ("リョ", "ryo", "ryo"),
    ("ギャ", "gya", "gya"),
    ("ギュ", "gyu", "gyu"),
    ("ギョ", "gyo", "gyo"),
    ("ジャ", "ja", "zya"),
    ("ジュ", "ju", "zyu"),
    ("ジョ", "jo", "zyo"),
    ("ヂャ", "ja", "zya"),
    ("ヂュ", "ju", "zyu"),
    ("ヂョ", "jo", "zyo"),
    ("ビャ", "bya", "bya"),
    ("ビュ", "byu", "byu"),
    ("ビョ", "byo", "byo"),
    ("ピャ", "pya", "pya"),
    ("ピュ", "pyu", "pyu"),
    ("ピョ", "pyo", "pyo"),
    // 外来音（ファイヤー・ウィンディなど実データに存在する）
    ("ファ", "fa", "fa"),
    ("フィ", "fi", "fi"),
    ("フェ", "fe", "fe"),
    ("フォ", "fo", "fo"),
    ("フュ", "fyu", "fyu"),
    ("ウィ", "wi", "wi"),
    ("ウェ", "we", "we"),
    ("ウォ", "wo", "wo"),
    ("ヴァ", "va", "va"),
    ("ヴィ", "vi", "vi"),
    ("ヴェ", "ve", "ve"),
    ("ヴォ", "vo", "vo"),
    ("ティ", "ti", "ti"),
    ("ディ", "di", "di"),
    ("トゥ", "tu", "tu"),
    ("ドゥ", "du", "du"),
    ("シェ", "she", "sye"),
    ("ジェ", "je", "zye"),
    ("チェ", "che", "tye"),
    ("ツァ", "tsa", "tsa"),
    ("ツィ", "tsi", "tsi"),
    ("ツェ", "tse", "tse"),
    ("ツォ", "tso", "tso"),
    ("クァ", "kwa", "kwa"),
    ("グァ", "gwa", "gwa"),
```

- [ ] **Step 4: `to_romaji` を 2 文字優先の走査に書き換える**

`src/romaji.rs` の `to_romaji` 全体を次で置き換える。

```rust
fn to_romaji(katakana: &str, style: Style) -> String {
    let chars: Vec<char> = katakana.chars().collect();
    let mut out = String::new();
    let mut i = 0;

    while i < chars.len() {
        // 拗音・外来音は 2 文字で 1 音なので、単カナより先に引く
        let pair: Option<String> = (i + 1 < chars.len()).then(|| chars[i..i + 2].iter().collect());
        if let Some(r) = pair.as_deref().and_then(|p| lookup(p, style)) {
            out.push_str(r);
            i += 2;
            continue;
        }

        match lookup(&chars[i].to_string(), style) {
            Some(r) => out.push_str(r),
            // 変換表にない文字は落とさずそのまま通す
            None => out.push(chars[i]),
        }
        i += 1;
    }

    out
}
```

- [ ] **Step 5: テストが通ることを確認**

Run: `cargo test romaji::`
Expected: 7 tests passed

- [ ] **Step 6: コミット**

```bash
git add src/romaji.rs
git commit -m "feat: romaji に拗音と外来音の変換を追加

Co-Authored-By: Claude Opus 5 <noreply@anthropic.com>"
```

---

### Task 3: 促音（ッ）と長音（ー）

ッ は次の音の子音を重ね、ー は直前の母音を重ねる。どちらも単独では音を持たないため変換表ではなく走査ロジックで扱う。

**Files:**
- Modify: `src/romaji.rs`（`to_romaji` を書き換え、`is_vowel` を追加）
- Test: `src/romaji.rs` の `#[cfg(test)] mod tests`

**Interfaces:**
- Consumes: Task 2 の `to_romaji` / `lookup` / `Style`
- Produces: シグネチャの変更なし。`variants` の挙動が拡張されるのみ。

- [ ] **Step 1: 失敗するテストを書く**

`src/romaji.rs` の `mod tests` に追加する。

```rust
    #[test]
    fn test_sokuon() {
        // ッ は次の音の子音を重ねる
        assert_eq!(variants("バッフロン"), vec!["baffuron", "bahhuron"]);
        assert_eq!(variants("ジェット"), vec!["jetto", "zyetto"]);
    }

    #[test]
    fn test_chouon() {
        // ー は直前の母音を重ねる
        assert_eq!(variants("ブースター"), vec!["buusutaa"]);
        assert_eq!(variants("サンダー"), vec!["sandaa"]);
    }

    #[test]
    fn test_chouon_at_head_is_ignored() {
        // 直前に母音がない ー は伸ばす対象がないので無視する
        assert_eq!(variants("ーカ"), vec!["ka"]);
    }
```

- [ ] **Step 2: テストが失敗することを確認**

Run: `cargo test romaji::`
Expected: `test_sokuon` と `test_chouon` が FAIL（ッ と ー が変換表にないためそのまま出力され、`"バッフロン"` が `"baッfuron"` になる）

- [ ] **Step 3: `to_romaji` を書き換え、`is_vowel` を追加**

`src/romaji.rs` の `to_romaji` 全体を次で置き換え、直前に `is_vowel` を追加する。

```rust
fn is_vowel(c: char) -> bool {
    matches!(c, 'a' | 'i' | 'u' | 'e' | 'o')
}

fn to_romaji(katakana: &str, style: Style) -> String {
    let chars: Vec<char> = katakana.chars().collect();
    let mut out = String::new();
    let mut i = 0;
    // 直前に ッ があったか。子音を重ねる対象は次の音なので持ち越す
    let mut sokuon = false;

    while i < chars.len() {
        let c = chars[i];

        if c == 'ッ' {
            sokuon = true;
            i += 1;
            continue;
        }

        if c == 'ー' {
            if let Some(v) = out.chars().last().filter(|v| is_vowel(*v)) {
                out.push(v);
            }
            i += 1;
            continue;
        }

        // 拗音・外来音は 2 文字で 1 音なので、単カナより先に引く
        let pair: Option<String> = (i + 1 < chars.len()).then(|| chars[i..i + 2].iter().collect());
        let (romaji, consumed) = match pair.as_deref().and_then(|p| lookup(p, style)) {
            Some(r) => (r, 2),
            None => match lookup(&c.to_string(), style) {
                Some(r) => (r, 1),
                // 変換表にない文字は落とさずそのまま通す
                None => {
                    out.push(c);
                    sokuon = false;
                    i += 1;
                    continue;
                }
            },
        };

        if sokuon {
            sokuon = false;
            // 母音始まりの音には重ねる子音がない
            if let Some(first) = romaji.chars().next().filter(|f| !is_vowel(*f)) {
                out.push(first);
            }
        }

        out.push_str(romaji);
        i += consumed;
    }

    out
}
```

- [ ] **Step 4: テストが通ることを確認**

Run: `cargo test romaji::`
Expected: 10 tests passed（Task 1・2 のテストも通ること）

- [ ] **Step 5: コミット**

```bash
git add src/romaji.rs
git commit -m "feat: romaji に促音と長音の変換を追加

Co-Authored-By: Claude Opus 5 <noreply@anthropic.com>"
```

---

### Task 4: skim のマッチ対象にローマ字を含め、表示からは隠す

`PokemonItem` に `match_text` を持たせ、`text()` はそれを返す。`display()` は可視部だけを描画し、ハイライト位置を可視部の文字数でクリップする。

**Files:**
- Modify: `src/romaji.rs`（`variants` の `#[allow(dead_code)]` を削除）
- Modify: `src/interactive.rs:16-31`（`PokemonItem` 定義と `SkimItem` 実装）
- Modify: `src/interactive.rs:90-99`（`run_skim_selection` のアイテム生成）
- Modify: `src/interactive.rs:226-263`（既存テスト `test_pokemon_item_text` / `test_pokemon_item_preview` の追随修正）
- Test: `src/interactive.rs` の `#[cfg(test)] mod tests`

**Interfaces:**
- Consumes: `crate::romaji::variants(katakana: &str) -> Vec<String>`（Task 1-3）
- Produces: `PokemonItem { japanese: String, english: String, display: String, match_text: String }`。Task 5 が `english` を `output()` から返す。

- [ ] **Step 1: 失敗するテストを書く**

`src/interactive.rs` の `mod tests` に追加する。既存の `use super::*;` で `DisplayContext` / `Matches` / `AnsiString` は skim の prelude 経由で解決される。

```rust
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
        // 隠しローマ字の位置だけにマッチした場合、可視部にハイライトは付かない
        let hidden = item.display.chars().count() + 2;
        let context = DisplayContext {
            text: &item.match_text,
            score: 0,
            matches: Matches::CharIndices(&[hidden]),
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
```

- [ ] **Step 2: テストが失敗することを確認**

Run: `cargo test interactive::`
Expected: コンパイルエラー（`struct 'PokemonItem' has no field named 'match_text'`）

- [ ] **Step 3: `PokemonItem` と `SkimItem` 実装を書き換える**

`src/interactive.rs:16-31` を次で置き換える。

```rust
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

    fn preview(&self, _context: PreviewContext) -> ItemPreview {
        ItemPreview::Text(format!("日本語: {}\n英語: {}", self.japanese, self.english))
    }
}
```

- [ ] **Step 4: アイテム生成でローマ字を付与する**

`src/interactive.rs:90-99` の `let items: Vec<Arc<dyn SkimItem>> = ...` を次で置き換える。

```rust
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
```

- [ ] **Step 5: `variants` の `#[allow(dead_code)]` を削除**

`src/romaji.rs` の `variants` から次の行を削除する。呼び出し元ができたため不要になる。

```rust
#[allow(dead_code)] // Task 4 で interactive.rs から使用する
```

- [ ] **Step 6: 既存テストを追随修正**

`src/interactive.rs` の `test_pokemon_item_text`（226-235 行目）を削除する。Step 1 で追加した `test_text_contains_romaji` が置き換える。

`test_pokemon_item_preview`（237-263 行目）の `PokemonItem` 構築を `create_test_item()` の呼び出しに差し替え、アサーションを合わせる。

```rust
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
```

- [ ] **Step 7: テストが通ることを確認**

Run: `cargo test`
Expected: 全テスト PASS

- [ ] **Step 8: sprites 機能付きでも警告なくビルドできることを確認**

Run: `cargo clippy --all-targets --all-features -- -D warnings`
Expected: 警告 0 件で終了

- [ ] **Step 9: コミット**

```bash
git add src/interactive.rs src/romaji.rs
git commit -m "feat: skim のマッチ対象にローマ字を含め、一覧表示からは隠す

Co-Authored-By: Claude Opus 5 <noreply@anthropic.com>"
```

---

### Task 5: `output()` で選択結果の英名を取得する

表示文字列を `" → "` で分割して英名を取り出している箇所を、`SkimItem::output()` に置き換える。表示書式の変更で壊れる箇所を 1 つ減らす。

**Files:**
- Modify: `src/interactive.rs`（`SkimItem` 実装に `output()` を追加）
- Modify: `src/interactive.rs:135-165`（`run_skim_selection` の結果処理）
- Test: `src/interactive.rs` の `#[cfg(test)] mod tests`

**Interfaces:**
- Consumes: Task 4 の `PokemonItem`（`english` フィールド）
- Produces: なし（最終タスク）

- [ ] **Step 1: 失敗するテストを書く**

`src/interactive.rs` の `mod tests` に追加する。

```rust
    #[test]
    fn test_output_returns_english_name() {
        // 確定時の返り値は表示文字列ではなく英名そのもの
        assert_eq!(create_test_item().output(), "Bulbasaur");
    }
```

- [ ] **Step 2: テストが失敗することを確認**

Run: `cargo test interactive::test_output_returns_english_name`
Expected: FAIL（デフォルト実装の `output()` が `text()` を返すため `"フシギダネ → Bulbasaur fushigidane husigidane"` になる）

- [ ] **Step 3: `output()` を実装**

`src/interactive.rs` の `impl SkimItem for PokemonItem` 内、`preview` の前に追加する。

```rust
    fn output(&self) -> std::borrow::Cow<'_, str> {
        self.english.as_str().into()
    }
```

- [ ] **Step 4: テストが通ることを確認**

Run: `cargo test interactive::test_output_returns_english_name`
Expected: PASS

- [ ] **Step 5: `run_skim_selection` の結果処理を書き換える**

`src/interactive.rs:135-165`（`if let Some(item) = selected_items.selected_items.first() {` から関数末尾の `Ok(None)` まで）を次で置き換える。

```rust
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
```

- [ ] **Step 6: 全テストと lint を確認**

Run: `cargo test && cargo clippy --all-targets --all-features -- -D warnings && cargo fmt --all -- --check`
Expected: すべて成功

- [ ] **Step 7: 実際に動かして確認**

Run: `cargo run --features sprites`

対話 UI で `fushigidane` と入力し、フシギダネ → Bulbasaur が候補に残ることを確認する。続けて `husigi`、`buusutaa` ではなく `busuta`（ブースター）でも絞り込めることを確認する。一覧にローマ字が表示されていないことも目視で確認する。

- [ ] **Step 8: コミット**

```bash
git add src/interactive.rs
git commit -m "refactor: 選択結果の英名を output() から取得する

表示文字列を ' → ' で分割するパースをやめ、表示書式の変更で
壊れる箇所を減らす。

Co-Authored-By: Claude Opus 5 <noreply@anthropic.com>"
```

---

## 実装後に残る既知の制約

設計ドキュメントに記載のとおり、ローマ字が効くのは引数なしで `poke-lookup` を起動したとき（`select_from_all` 経由）のみ。`poke-lookup fushigi` は `SearchService::search_partial` がカタカナのみを対象とするため候補 0 件になる。今回のスコープ外。
