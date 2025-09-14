# 要件定義

## 目的

* 日本語名（カタカナ）を入力すると **PokéAPI 準拠**の英名を返す CLI。
* 出力結果は **Pokemiro** でそのまま利用できる。

## スコープ

* **種（species）レベルのみ**対応。フォームは対象外（PokéAPI 側の日本語未整備のため）。
* 方向は現状 **日本語 → 英語** のみ（将来拡張の余地は残すが要件外）。

## データ

* **ソース**：`pokemon-species/{id}` の `names` 配列

  * `language == "ja-Hrkt"` → 日本語名
  * `language == "en"` → 英名
* **配布形式**：`names.json`（単一ファイル）

  ```json
  {
    "schema_version": 1,
    "generated_at": "YYYY-MM-DDThh:mm:ssZ",
    "count": <int>,
    "entries": [
      { "ja": "ピカチュウ", "en": "Pikachu" },
      ...
    ]
  }
  ```
* **設置場所**：デフォルトは XDG データディレクトリ（例：`~/.local/share/poke-lookup/names.json`）。
  オプションで `--dict <path>` などの上書き可。

## データ更新（CI 前提・毎月）

* GitHub Actions で **毎月** 自動実行し `names.json` を生成・公開（Releases など）。
* 併せて **SHA256** を公開。`schema_version` / `generated_at` / `count` を埋め込む。
* ユーザー側は `poke-lookup update` でダウンロード → **HTTPS+SHA256検証** → アトミック置換。
* 緊急用に `--online` を用意（ユーザー端末で PokéAPI を直接クロールして生成）。通常は非推奨（負荷配慮）。

## CLI 仕様

* **基本**：`poke-lookup <日本語名>`

  1. **完全一致が1件** → 即確定して英名を標準出力
  2. それ以外 → **インタラクティブ選択**（`skim` 利用）

     * インクリメンタル検索／**Emacs バインド**（`C-n`/`C-p` など）対応
     * ユーザーがEnterキーで確定
  3. 候補0件 → 「候補が見つかりませんでした」＋終了コード `2`
* **更新**：`poke-lookup update [--online] [--source <URL>] [--verify-sha256 <HEX>] [--dry-run]`
* **終了コード**：`0` 成功 / `2` 候補なし / `130` ユーザーキャンセル

## ヘルプ（例）

```
poke-lookup - 日本語名からポケモンの英名を取得するツール (PokéAPI準拠)

USAGE:
    poke-lookup <日本語名>
    poke-lookup update [--online] [--source <URL>] [--verify-sha256 <HEX>] [--dry-run]

ARGS:
    <日本語名>   ポケモンの日本語名（カタカナ、種レベル）

OPTIONS:
    -h, --help       このヘルプを表示
    -V, --version    バージョンを表示
        --dict PATH  names.json の明示パス

SUBCOMMANDS:
    update           names.json を更新（既定はCI配布を取得）
        --online         PokéAPI を直接クロールして生成（通常は不要）
        --source <URL>   CI配布のURLを上書き
        --verify-sha256  取得ファイルの検証
        --dry-run        置換せず検証のみ
```

## パフォーマンス／実装方針（非機能要件）

* 起動時に `names.json` を読み込み **HashMap化（O(1) 検索）**。件数規模（\~1,000）では十分高速。
* インタラクティブは `skim` を採用（fzf ライク・Emacs キーバインド・文字列絞り込み）。
* ログは控えめ（標準出力は英名のみ、メッセージは標準エラーへ）。

## セキュリティ／信頼性

* 更新時は **HTTPS + SHA256** 検証。失敗時は既存ファイルを保持。
* `schema_version` の不一致や `count` 異常時は更新を拒否。
* 一時ファイル → `rename()` のアトミック置換で破損回避。

## 非スコープ

* フォーム（ロトム・アローラ・ガラル等）の日本語対応
* 英語→日本語の逆引き

ご指摘ありがとうございます 🙏
いま提示した最終要件まとめでは、**「Rust を採用する」という前提」が明示されていませんでした**。
確かに実装技術も要件に含めておくべきです。

---

# 修正：実装技術も含めた要件定義

## 目的

* 日本語名（カタカナ）を入力すると **PokéAPI 準拠**の英名を返す CLI。
* 出力結果は **Pokemiro** でそのまま利用できる。
* **実装言語は Rust**。

## 実装要件（技術選択）

* **言語**: Rust

  * バイナリ配布が容易
  * 高速・省メモリ
* **主なライブラリ**

  * `serde_json`：`names.json` の読み書き
  * `skim`：インタラクティブ選択（fzfライク、Emacsバインド対応）
  * `clap`：CLI 引数・ヘルプ管理
  * `reqwest`：`update` 時のファイル取得
  * `sha2`：SHA256 検証
* **配布形態**:

  * GitHub Releases にクロスビルドしたバイナリを配置
  * Cargo からインストール可能にするのも検討（`cargo install poke-lookup`）

