# タイプ検索 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 対話画面（skim）の絞り込み欄で、ポケモンをタイプ（`ほのお` / `fire` 等）でも絞り込めるようにする。

**Architecture:** `names.json` の各エントリに英語タイプ slug 配列 `types` を焼き込み（schema v2）、Rust 側は slug→日本語の静的テーブルでトークン（`ほのお fire`）を生成、skim の `match_text` にローマ字と同じ隠しトークンとして混ぜる。表示は変えない。

**Tech Stack:** Rust（clap / skim / serde）、データ生成は Python（`.github/scripts/fetch-pokemon-data.py`）。

## Global Constraints

- タイプ名トークンは **日本語＋英語 slug** のみ。タイプ名のローマ字（`honoo`）は含めない。
- 一覧（skim）の **表示（`display`）は現状維持**。タイプは隠しトークンとして `match_text` にのみ入れる。
- タイプ絞り込みは **対話画面（skim）内でのみ** 効く。引数検索（`search_partial`）は変更しない。
- slug→日本語の静的テーブルは **feature 非依存**（`sprites` を切っても使える）。
- `schema_version` は **2**。`models.rs` の `EXPECTED_VERSION` も 2。
- タイプ slug の並びは PokéAPI の `slot` 昇順。
- 既存テストの `NameEntry { .. }` リテラルと `schema_version` は本計画の変更に追随させる。

---

### Task 1: slug→日本語タイプ名テーブルを feature 非依存モジュールへ切り出し

`info.rs` の `type_ja`（`#[cfg(feature = "sprites")]` でゲート、`src/info.rs:14-38`）を、`sprites` に依存しない共有モジュール `src/pokemon_type.rs` へ移す。挙動は不変のリファクタ。

**Files:**
- Create: `src/pokemon_type.rs`
- Modify: `src/main.rs:1-12`（`mod pokemon_type;` を feature ゲート無しで追加）
- Modify: `src/info.rs:14-38`（ローカル `type_ja` を削除し `crate::pokemon_type::type_ja` を使う）

**Interfaces:**
- Produces: `pub fn type_ja(slug: &str) -> Option<&'static str>`（18タイプ、未知は `None`）

- [ ] **Step 1: 新モジュールにテストを書く**

`src/pokemon_type.rs`:

```rust
/// タイプの英語スラッグ → 日本語名。18種は固定なので追加APIを叩かずここで引く。
/// sprites 機能とタイプ検索の双方から使うため feature ゲートしない。
pub fn type_ja(slug: &str) -> Option<&'static str> {
    Some(match slug {
        "normal" => "ノーマル",
        "fire" => "ほのお",
        "water" => "みず",
        "electric" => "でんき",
        "grass" => "くさ",
        "ice" => "こおり",
        "fighting" => "かくとう",
        "poison" => "どく",
        "ground" => "じめん",
        "flying" => "ひこう",
        "psychic" => "エスパー",
        "bug" => "むし",
        "rock" => "いわ",
        "ghost" => "ゴースト",
        "dragon" => "ドラゴン",
        "dark" => "あく",
        "steel" => "はがね",
        "fairy" => "フェアリー",
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_type_ja_known_and_unknown() {
        assert_eq!(type_ja("fire"), Some("ほのお"));
        assert_eq!(type_ja("flying"), Some("ひこう"));
        assert_eq!(type_ja("stellar"), None);
    }
}
```

- [ ] **Step 2: `main.rs` にモジュール宣言を追加**

`src/main.rs` の他 `mod` 宣言の並び（`src/main.rs:2-12`）に、feature ゲート無しで追加:

```rust
mod pokemon_type;
```

- [ ] **Step 3: `info.rs` からローカル `type_ja` を削除し共有版を使う**

`src/info.rs:14-38` のローカル `type_ja` 関数定義（doc コメント含む）を削除。`fetch` 内の呼び出し（`src/info.rs:224`）を差し替える:

```rust
                ja: crate::pokemon_type::type_ja(&slot.type_ref.name)
                    .map(str::to_string)
                    .unwrap_or_else(|| slot.type_ref.name.clone()),
```

`info.rs` の既存テスト `test_type_ja_known_and_unknown`（`src/info.rs:432-436`）は Task1 の新モジュールへ移したので削除する。

- [ ] **Step 4: 両 feature 構成でビルド・テスト**

Run: `cargo test && cargo test --no-default-features`
Expected: PASS（新規 `pokemon_type::tests::test_type_ja_known_and_unknown` を含む。`sprites` 有効時は `info.rs` が共有 `type_ja` を使ってビルド成功）

- [ ] **Step 5: Commit**

```bash
git add src/pokemon_type.rs src/main.rs src/info.rs
git commit -m "refactor: タイプslug→日本語テーブルをfeature非依存モジュールへ切り出し"
```

---

### Task 2: `NameEntry` に `types` を追加し schema を v2 に上げる

**Files:**
- Modify: `src/models.rs:19-28`（`types` フィールド追加）
- Modify: `src/models.rs:41`（`EXPECTED_VERSION` を 2 に）
- Modify: `src/models.rs` テスト群（`schema_version` と `NameEntry` リテラル追随）
- Modify: `src/data.rs:104-122`（`create_test_data` の `schema_version` と `NameEntry`）
- Modify: `src/search.rs:130-147`（`test_from_loader` の `schema_version` と `NameEntry`）

**Interfaces:**
- Produces: `NameEntry { ja: String, en: String, id: Option<u32>, types: Vec<String> }`。`types` は `#[serde(default)]`＋`#[serde(skip_serializing_if = "Vec::is_empty")]`。

- [ ] **Step 1: types のデシリアライズを検証する失敗テストを書く**

`src/models.rs` の `mod tests` に追加:

```rust
    #[test]
    fn test_deserialize_types() {
        let json = r#"{
            "schema_version": 2,
            "generated_at": "2025-01-01T00:00:00Z",
            "count": 2,
            "entries": [
                {"ja": "リザードン", "en": "Charizard", "id": 6, "types": ["fire", "flying"]},
                {"ja": "ピカチュウ", "en": "Pikachu"}
            ]
        }"#;

        let dict: NameDictionary = serde_json::from_str(json).unwrap();
        assert_eq!(dict.entries[0].types, vec!["fire", "flying"]);
        // types キーが無い旧形式のエントリは空ベクタになる（#[serde(default)]）
        assert!(dict.entries[1].types.is_empty());
    }
```

- [ ] **Step 2: テストを実行して失敗を確認**

Run: `cargo test --lib models::tests::test_deserialize_types`
Expected: コンパイルエラー（`NameEntry` に `types` フィールドが無い）

- [ ] **Step 3: `NameEntry` に `types` を追加し schema を 2 に**

`src/models.rs:19-28` を変更:

```rust
/// 個別のポケモン名エントリ
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct NameEntry {
    /// 日本語名（カタカナ）
    pub ja: String,
    /// 英名
    pub en: String,
    /// ポケモンID（スプライト表示用）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<u32>,
    /// タイプの英語スラッグ（slot 昇順）。旧データには無いので default で空
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub types: Vec<String>,
}
```

`src/models.rs:41` の定数を変更:

```rust
        const EXPECTED_VERSION: u32 = 2;
```

- [ ] **Step 4: 既存テストの `schema_version` と `NameEntry` リテラルを追随**

`src/models.rs` 内テストで `schema_version: 1` を使っている箇所（`test_deserialize_name_dictionary` の JSON、`test_to_hashmap`、`test_validate_schema`、`test_validate_count`、`test_validate`、`test_validate_entries_empty_names`、`test_validate_entries_zero_count`、`test_validate_entries_exceed_limit`）をすべて `2` に更新する。`test_validate_schema` の「不正値」検証は `dict.schema_version = 3;` に変える（2 が正常値になったため）:

```rust
        dict.schema_version = 3;
        assert!(dict.validate_schema().is_err());
```

`src/models.rs` 内の全 `NameEntry { .. }` リテラルに `types: vec![]` を追加する（`test_to_hashmap` の2件、`test_validate_count` の2件、`test_validate` の1件、`test_validate_entries_empty_names` の1件）。例:

```rust
                NameEntry {
                    ja: "ピカチュウ".to_string(),
                    en: "Pikachu".to_string(),
                    id: None,
                    types: vec![],
                },
```

`src/data.rs:104-122` `create_test_data`: `schema_version: 1` → `2`、2件の `NameEntry` に `types: vec![]` を追加。同ファイルの `test_load_dictionary_success`（`src/data.rs:160`）の `assert_eq!(result.schema_version, 1);` を `2` に更新。

`src/search.rs:130-147` `test_from_loader`: `schema_version: 1` → `2`、2件の `NameEntry` に `types: vec![]` を追加。

- [ ] **Step 5: 全テスト実行**

Run: `cargo test && cargo test --no-default-features`
Expected: PASS

- [ ] **Step 6: Commit**

```bash
git add src/models.rs src/data.rs src/search.rs
git commit -m "feat: NameEntryにtypesを追加しschemaをv2に更新"
```

---

### Task 3: `SearchService` がタイプを保持し `type_tokens` を提供

**Files:**
- Modify: `src/models.rs`（`NameDictionary` に `to_type_map` を追加）
- Modify: `src/search.rs:6-42`（`type_map` フィールドと構築）
- Modify: `src/search.rs`（`type_tokens` メソッド追加）

**Interfaces:**
- Consumes: `NameEntry.types`（Task 2）、`crate::pokemon_type::type_ja`（Task 1）
- Produces:
  - `NameDictionary::to_type_map(&self) -> HashMap<String, Vec<String>>`（ja → types）
  - `SearchService::type_tokens(&self, japanese_name: &str) -> String`（例 `"ほのお fire ひこう flying"`、無ければ空文字）

- [ ] **Step 1: `to_type_map` の失敗テストを書く**

`src/models.rs` の `mod tests` に追加:

```rust
    #[test]
    fn test_to_type_map() {
        let dict = NameDictionary {
            schema_version: 2,
            generated_at: Utc::now(),
            count: 1,
            entries: vec![NameEntry {
                ja: "リザードン".to_string(),
                en: "Charizard".to_string(),
                id: Some(6),
                types: vec!["fire".to_string(), "flying".to_string()],
            }],
        };

        let map = dict.to_type_map();
        assert_eq!(
            map.get("リザードン"),
            Some(&vec!["fire".to_string(), "flying".to_string()])
        );
    }
```

- [ ] **Step 2: テスト実行で失敗確認**

Run: `cargo test --lib models::tests::test_to_type_map`
Expected: コンパイルエラー（`to_type_map` 未定義）

- [ ] **Step 3: `to_type_map` を実装**

`src/models.rs` の `impl NameDictionary`（`to_hashmap` の直後）に追加:

```rust
    /// エントリを ja → types の HashMap に変換（タイプトークン生成用）
    pub fn to_type_map(&self) -> HashMap<String, Vec<String>> {
        self.entries
            .iter()
            .map(|entry| (entry.ja.clone(), entry.types.clone()))
            .collect()
    }
```

- [ ] **Step 4: `SearchService::type_tokens` の失敗テストを書く**

`src/search.rs` の `mod tests` に追加（`create_test_service` は `type_map` 未対応なので、このテストは専用のコンストラクタで組む）:

```rust
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

        assert_eq!(service.type_tokens("リザードン"), "ほのお fire ひこう flying");
        // types 無し・未登録は空文字
        assert_eq!(service.type_tokens("ピカチュウ"), "");
    }
```

- [ ] **Step 5: テスト実行で失敗確認**

Run: `cargo test --lib search::tests::test_type_tokens`
Expected: コンパイルエラー（`from_maps` / `type_tokens` 未定義）

- [ ] **Step 6: `SearchService` に `type_map` と `type_tokens` を実装**

`src/search.rs:6-22` を変更:

```rust
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
```

既存の `from_name_map`（`src/search.rs:26-28`）は `type_map` を空にして温存し、汎用の `from_maps` を追加:

```rust
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
    /// 各 slug を「日本語名 slug」に展開して空白区切りで並べる（例: "ほのお fire"）。
    /// 未知 slug は slug のみ。types が無ければ空文字。
    pub fn type_tokens(&self, japanese_name: &str) -> String {
        self.type_map
            .get(japanese_name)
            .map(|slugs| {
                slugs
                    .iter()
                    .map(|slug| match crate::pokemon_type::type_ja(slug) {
                        Some(ja) => format!("{} {}", ja, slug),
                        None => slug.clone(),
                    })
                    .collect::<Vec<_>>()
                    .join(" ")
            })
            .unwrap_or_default()
    }
```

`create_test_service`（`src/search.rs:87-96`）は `SearchService { name_map }` を直接構築しているので、`type_map: HashMap::new()` を追加する。

- [ ] **Step 7: 全テスト実行**

Run: `cargo test && cargo test --no-default-features`
Expected: PASS

- [ ] **Step 8: Commit**

```bash
git add src/models.rs src/search.rs
git commit -m "feat: SearchServiceにタイプ保持とtype_tokensを追加"
```

---

### Task 4: `PokemonItem` の `match_text` にタイプトークンを混ぜる

**Files:**
- Modify: `src/interactive.rs:77-88`（`PokemonItem::new` にタイプトークン引数）
- Modify: `src/interactive.rs:208-211`（`run_skim_selection` の item 生成）
- Modify: `src/interactive.rs:364-366`（テストヘルパ `create_test_item`）

**Interfaces:**
- Consumes: `SearchService::type_tokens`（Task 3）
- Produces: `PokemonItem::new(ja: &str, en: &str, type_tokens: &str) -> PokemonItem`

- [ ] **Step 1: タイプトークンが `match_text` に入り `display` には入らない失敗テストを書く**

`src/interactive.rs` の `mod tests` に追加:

```rust
    #[test]
    fn test_match_text_contains_type_tokens() {
        let item = PokemonItem::new("リザードン", "Charizard", "ほのお fire ひこう flying");
        let text = item.text();
        assert!(text.contains("ほのお"));
        assert!(text.contains("fire"));
        // 表示はタイプを含まない
        assert_eq!(item.display, "リザードン → Charizard");
    }

    #[test]
    fn test_match_text_without_type_tokens() {
        // 空トークンでも従来通り（末尾に余計な空白を足さない）
        let item = PokemonItem::new("ピカチュウ", "Pikachu", "");
        assert!(item.text().contains("ピカチュウ"));
        assert!(item.text().contains("pikachu"));
    }
```

- [ ] **Step 2: テスト実行で失敗確認**

Run: `cargo test --lib interactive::tests::test_match_text_contains_type_tokens`
Expected: コンパイルエラー（`PokemonItem::new` の引数が2個）

- [ ] **Step 3: `PokemonItem::new` にタイプトークンを追加**

`src/interactive.rs:77-88` を変更:

```rust
impl PokemonItem {
    fn new(ja: &str, en: &str, type_tokens: &str) -> Self {
        let display = format!("{} → {}", ja, en);
        let romaji = crate::romaji::variants(ja).join(" ");
        // タイプはローマ字と同じく match_text にだけ載せる隠しトークン。
        // 空のときに末尾空白を足さないよう分岐する
        let match_text = if type_tokens.is_empty() {
            format!("{} {}", display, romaji)
        } else {
            format!("{} {} {}", display, romaji, type_tokens)
        };
        Self {
            japanese: ja.to_string(),
            english: en.to_string(),
            match_text,
            display,
        }
    }
}
```

- [ ] **Step 4: `run_skim_selection` の item 生成でタイプトークンを渡す**

`src/interactive.rs:208-211` を変更:

```rust
        let items: Vec<Arc<dyn SkimItem>> = candidates
            .iter()
            .map(|(ja, en)| {
                let type_tokens = self.search_service.type_tokens(ja);
                Arc::new(PokemonItem::new(ja, en, &type_tokens)) as Arc<dyn SkimItem>
            })
            .collect();
```

- [ ] **Step 5: テストヘルパを新シグネチャに追随**

`src/interactive.rs:364-366` の `create_test_item` を変更:

```rust
    fn create_test_item() -> PokemonItem {
        PokemonItem::new("フシギダネ", "Bulbasaur", "")
    }
```

- [ ] **Step 6: 全テスト実行**

Run: `cargo test && cargo test --no-default-features`
Expected: PASS（既存の `test_text_contains_romaji`・`test_display_*` も空トークンで従来どおり通る）

- [ ] **Step 7: Commit**

```bash
git add src/interactive.rs
git commit -m "feat: skimのmatch_textにタイプトークンを追加"
```

---

### Task 5: データ生成スクリプトが `types` を出力（schema v2）

**Files:**
- Modify: `.github/scripts/fetch-pokemon-data.py`

**Interfaces:**
- Produces: 各エントリに `types`（英語 slug 配列、slot 昇順）、出力の `schema_version` は 2

- [ ] **Step 1: `extract_types` ヘルパを追加**

`.github/scripts/fetch-pokemon-data.py` の `slug_to_en`（`.github/scripts/fetch-pokemon-data.py:83-85`）付近に追加:

```python
def extract_types(pokemon_data: dict) -> List[str]:
    """pokemon データからタイプの英語スラッグを slot 昇順で取り出す"""
    slots = sorted(pokemon_data.get('types', []), key=lambda t: t['slot'])
    return [t['type']['name'] for t in slots]
```

- [ ] **Step 2: base（種）のデフォルト個体からタイプを取得**

タイプは `pokemon-species` に無く `pokemon` 側にあるため、デフォルト個体を辿る。`main()` の species ループ（`.github/scripts/fetch-pokemon-data.py:200-204`）の `if name_pair:` ブロックを変更:

```python
            # 名前ペアを抽出
            name_pair = get_name_pair(species_data)
            if name_pair:
                # タイプは pokemon 側にしか無いのでデフォルト個体を辿る
                default_url = next(
                    (v['pokemon']['url']
                     for v in species_data.get('varieties', [])
                     if v.get('is_default')),
                    None,
                )
                if default_url:
                    name_pair['types'] = extract_types(fetch_json(default_url))
                entries.append(name_pair)
                variety_refs.extend(get_variety_refs(species_data, name_pair['ja']))
```

- [ ] **Step 3: フォルムのエントリにタイプを付与**

`fetch_form_entry` の return（`.github/scripts/fetch-pokemon-data.py:105-111`）に `types` を追加:

```python
    return {
        'ja': compose_ja(base_ja, form_ja),
        'en': form_en or slug_to_en(pokemon_data['name']),
        'id': pokemon_data['id'],
        'types': extract_types(pokemon_data),
        'slug': pokemon_data['name'],
        'species_slug': pokemon_data['species']['name'],
    }
```

- [ ] **Step 4: 出力の schema_version を 2 に**

`.github/scripts/fetch-pokemon-data.py:253-258` の `output` 辞書を変更:

```python
    output = {
        'schema_version': 2,
        'generated_at': datetime.now(timezone.utc).strftime('%Y-%m-%dT%H:%M:%SZ'),
        'count': len(entries),
        'entries': entries
    }
```

- [ ] **Step 5: `extract_types` をローカルで検証**

Run:
```bash
python3 -c "
import importlib.util, sys
spec = importlib.util.spec_from_file_location('f', '.github/scripts/fetch-pokemon-data.py')
m = importlib.util.module_from_spec(spec); spec.loader.exec_module(m)
sample = {'types': [{'slot': 2, 'type': {'name': 'flying'}}, {'slot': 1, 'type': {'name': 'fire'}}]}
assert m.extract_types(sample) == ['fire', 'flying'], m.extract_types(sample)
assert m.extract_types({}) == []
print('ok')
"
```
Expected: `ok`（slot 昇順で `['fire', 'flying']`、空データで `[]`）

- [ ] **Step 6: Commit**

```bash
git add .github/scripts/fetch-pokemon-data.py
git commit -m "feat: データ生成でtypesを出力しschemaをv2に"
```

---

### Task 6: README にタイプ絞り込みを追記

**Files:**
- Modify: `README.md`

- [ ] **Step 1: 特徴とローマ字絞り込みの節に追記**

`README.md` の「## 使い方」内、「### ローマ字での絞り込み」（`README.md:103-115`）の直後に節を追加:

```markdown
### タイプでの絞り込み

引数なしで起動した全候補選択や、部分一致の候補選択の画面では、タイプ名でも絞り込めます。日本語（`ほのお`）でも英語（`fire`）でも引けます。

```bash
$ poke-lookup
# 絞り込み入力に ほのお と打つと ほのおタイプのポケモンが残る
```

**引数に渡すタイプ名は対象外**です（ローマ字と同じ挙動）。タイプで絞る場合は対話画面の絞り込み欄に入力してください。
```

「## 特徴」の一覧（`README.md:12-20`）にも1行追加:

```markdown
- 🏷️ 対話画面でタイプ名（日本語/英語）による絞り込み
```

- [ ] **Step 2: データ更新の必要性を追記**

`README.md` の「## 初回セットアップ」で、schema v2 化により再取得が必要な旨は既存の `poke-lookup update` 手順で足りるため、追加変更は不要か軽微な注記のみとする。既存文面を確認し、タイプデータを含む旨を1文添える程度に留める。

- [ ] **Step 3: Commit**

```bash
git add README.md
git commit -m "docs: タイプでの絞り込みをREADMEに追記"
```

---

## Self-Review

**Spec coverage:**
- データ層に `types`／schema v2 → Task 2・Task 5 ✅
- slug→日本語 静的テーブル（feature 非依存） → Task 1 ✅
- `SearchService` がタイプ保持・トークン生成 → Task 3 ✅
- `match_text` に隠しトークン → Task 4 ✅
- 後方互換（`#[serde(default)]`／EXPECTED_VERSION=2） → Task 2 ✅
- README 追記 → Task 6 ✅

**Placeholder scan:** 各ステップに実コードを記載。Task 6 Step 2 のみ既存文面確認に依存するが、変更は「軽微な注記」と範囲を明示。

**Type consistency:**
- `type_ja(&str) -> Option<&'static str>`（Task 1）を Task 3 の `type_tokens` が使用 — 一致。
- `NameEntry.types: Vec<String>`（Task 2）を `to_type_map`（Task 3）が参照 — 一致。
- `SearchService::type_tokens(&self, &str) -> String`（Task 3）を Task 4 の `run_skim_selection` が使用 — 一致。
- `PokemonItem::new(&str, &str, &str)`（Task 4）— テストヘルパ・item 生成の両方を更新 — 一致。
