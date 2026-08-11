# タイプ検索 設計ドキュメント

- 日付: 2026-08-12
- ブランチ: `feature/type-search`
- 方式: A（対話画面の絞り込み欄にタイプを隠しトークンとして混ぜる）

## 目的

ポケモンをタイプ（ほのお・みず 等）で絞り込めるようにする。現状は日本語名／英名（と対話画面ではローマ字）でしか検索できない。

## 確定した要件

- **入力表記**: 日本語（`ほのお`）＋ 英語 slug（`fire`）の両方で絞れる。タイプ名のローマ字（`honoo`）は含めない。
- **表示**: 一覧（skim）の表示は現状維持。タイプは検索キーとしてだけ使う隠しトークン。
- **入口**: 方式Aの定義どおり、**対話画面（skim）の絞り込み欄でのみ**タイプ絞り込みが効く。引数 `poke-lookup ほのお` は従来どおり名前検索の対象で、タイプでは引けない。
- タイプ名（ひらがな）はポケモン名（カタカナ）と字種が異なるため、誤マッチが起きにくい。

## 非目標（YAGNI）

- 引数からのタイプ検索、タイプ専用サブコマンド、複数タイプの AND/OR 指定 UI。
- タイプの一覧表示・preview 表示。
- タイプ名ローマ字での検索。

## 設計

### 1. データ層（`.github/scripts/fetch-pokemon-data.py` / `names.json`）

各エントリにタイプの**英語 slug 配列**を追加する（`slot` 昇順）。

```json
{ "ja": "リザードン", "en": "Charizard", "id": 6, "types": ["fire", "flying"] }
```

- タイプは `pokemon` エンドポイントの `types`（各要素の `type.name` を `slot` 順に並べる）から取得する。`pokemon-species` には無い。
- **base（種）の取得追加**: 現状スクリプトは base では `pokemon-species` だけを見ており `pokemon` を叩いていない（`fetch-pokemon-data.py:196-204`）。`species_data['varieties']` の `is_default == true` の `pokemon.url` を辿って `types` を取得する。約1025リクエスト増。フォルム取得と同様に `ThreadPoolExecutor` で並列化する。
- **フォルム**: `fetch_form_entry` は既に `pokemon_data` を取得済み（`fetch-pokemon-data.py:93`）なので、そこから `types` を足すだけ。
- `schema_version` を **1 → 2** に上げる。旧データには types が無いため、`poke-lookup update` での再取得を促す（初回 update は元々必須）。

### 2. タイプ名マッピング（Rust 側の静的テーブル）

slug → 日本語名 の18件固定表を Rust に持つ（例: `fire → ほのお`）。`names.json` には日本語タイプ名を重複保存しない（18件は安定・不変に近い）。マッチトークン生成にのみ使う。

### 3. 検索UI層（`src/models.rs` / `src/search.rs` / `src/interactive.rs`）

- `NameEntry` に `#[serde(default)] pub types: Vec<String>` を追加。空配列でデシリアライズできるため、types 無しの旧データでも読める（後方互換）。
- `SearchService` がタイプ情報を保持する。現在の `HashMap<ja, en>`（完全一致・部分一致に使用）に加え、`ja → Vec<String>(types)` の対応を併設する。`all_entries` / `search_partial` の戻り値でタイプも渡せるようにする。
- `PokemonItem::new` で `match_text` にタイプトークンを追加する（ローマ字と同じ隠しトークン方式）。

  ```
  match_text = "リザードン → Charizard  fushigi... ほのお fire ひこう flying"
  display    = "リザードン → Charizard"      # 変更なし
  ```

  タイプトークンは各 slug について「日本語名 slug」を並べる（例: `ほのお fire ひこう flying`）。

### 4. 後方互換

- types が空のエントリはタイプトークンが付かず、名前検索は従来どおり。
- `#[serde(default)]` は「types キーが無い JSON でもデシリアライズが失敗しない」ための堅牢性であり、データ受け入れの判定ではない。受け入れの実ゲートは `schema_version`。
- `schema_version` を 2 に上げるため、`models.rs` の `EXPECTED_VERSION` を 2 に更新する。旧 v1 データは `validate_schema` で弾かれ、`poke-lookup update` を促される。

## テスト

- `PokemonItem::new` の `match_text` にタイプトークン（`ほのお` / `fire`）が含まれること。
- `display` はタイプを含まず従来どおりであること。
- `types` を持つエントリが JSON からデシリアライズできること／`types` 無し JSON も `#[serde(default)]` で読めること。
- slug → 日本語名マッピングの主要ケース。

## 影響ファイル

- `.github/scripts/fetch-pokemon-data.py`（types 取得・schema_version=2）
- `src/models.rs`（`types` フィールド・`EXPECTED_VERSION`）
- `src/search.rs`（タイプ保持・戻り値拡張）
- `src/interactive.rs`（`match_text` にタイプトークン）
- 新規: タイプ slug→日本語 の静的テーブル（`src/types.rs` 等）
- `README.md`（タイプ絞り込みの説明追記）
