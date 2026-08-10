# poke-lookup

日本語のポケモン名から PokéAPI 準拠の英名を取得する CLI ツール

## 概要

`poke-lookup` は、カタカナのポケモン名を入力すると対応する英名を返すコマンドラインツールです。
[Pokémiro](https://miro.com/marketplace/pokemiro/) などの他ツールと連携して使用できます。

## 特徴

- 🔍 高速な検索（HashMap による O(1) アクセス）
- 🎯 完全一致で即座に結果を返す
- 📝 部分一致時はインタラクティブ選択（skim 使用）
- ⌨️ インタラクティブ選択ではローマ字入力でも絞り込み可能
- 🖼️ ターミナル内スプライト表示（オプション機能）
- 🔊 鳴き声再生（オプション機能）
- 🔄 月次自動データ更新（GitHub Actions）
- 🔒 SHA256 によるデータ整合性チェック
- 🌐 PokéAPI 準拠のデータ

## インストール

### 前提条件

- Rust 1.70以上
- Git（クローン用）
- インターネット接続（初回セットアップ時のデータダウンロード用）
- Linux の場合のみ、鳴き声機能が依存する ALSA の開発ヘッダ（`libasound2-dev`）

### インストール手順

```bash
git clone https://github.com/cozy-corner/poke-lookup.git
cd poke-lookup
cargo install --path .
```

これにより `poke-lookup` コマンドがどこからでも実行可能になります。スプライト表示と鳴き声再生はデフォルトで有効なので、追加の指定は不要です。

https://github.com/user-attachments/assets/7f80ef19-2117-4c19-b8a8-96bdf55d6ee3

### 最小構成でのインストール

スプライトと鳴き声を無効にし、依存を減らした最小構成でビルドしたい場合：

```bash
cargo install --path . --no-default-features
```

個別に有効化したい場合は `--features sprites` / `--features cries` を組み合わせてください。

### 手動ビルド（開発用）

```bash
# 全機能付き（デフォルト）
cargo build --release

# 最小構成（スプライト・鳴き声なし）
cargo build --release --no-default-features
```

### 開発環境のセットアップ

開発に参加する場合は、Git hooksを設定することを推奨します：

```bash
# Git hooksの設定（pre-commit: format, lint, test）
./scripts/setup-hooks.sh
```

これにより、コミット時に自動的に以下のチェックが実行されます：
- `cargo fmt --check`: コードフォーマット
- `cargo clippy`: Lintチェック
- `cargo test`: テスト実行

## 初回セットアップ

**重要**: 初回実行前にデータファイルのダウンロードが必要です。

```bash
poke-lookup update
```

これにより、最新のポケモンデータ（names.json）がダウンロードされます。

## 使い方

### 基本的な使用方法

```bash
# 完全一致の場合、即座に英名を返す
$ poke-lookup ピカチュウ
Pikachu

# 部分一致の場合、インタラクティブ選択
$ poke-lookup フシギ
> フシギダネ
  フシギソウ
  フシギバナ
```

### ローマ字での絞り込み

引数なしで起動すると全候補からインタラクティブ選択になり、この画面ではローマ字でも絞り込めます。

```bash
$ poke-lookup
# 絞り込み入力に fushigidane と打つと フシギダネ が残る
> フシギダネ → Bulbasaur
```

ヘボン式・訓令式のどちらでも引けます（`fushigi` / `husigi`）。長音は省略しても構いません（`busuta` で ブースター）。

なお**引数に渡すローマ字は対象外**です。`poke-lookup fushigidane` は候補なしになります。ローマ字を使う場合は引数なしで起動してください。

### スプライト表示

ポケモンの画像をターミナル内に表示できます：

```bash
# 英名と一緒にスプライトを表示
$ poke-lookup ピカチュウ --show-sprite
Pikachu
[ピカチュウのスプライト画像がターミナルに表示]

# 短縮オプション
$ poke-lookup ピカチュウ -s
Pikachu
[ピカチュウのスプライト画像がターミナルに表示]

# インタラクティブ選択でもスプライト表示
$ poke-lookup フシギ -s
# 選択後にスプライトが表示されます
```

**対応ターミナル:**
- iTerm2（macOS）
- Kitty
- WezTerm
- その他の画像表示対応ターミナル

**注意:** スプライト機能はデフォルトで有効です。最小構成（`--no-default-features`）でビルドした場合のみ無効になります。

### 鳴き声再生

選択したポケモンの鳴き声を再生できます：

```bash
# 英名を出力して鳴き声を再生
$ poke-lookup ピカチュウ --play-cry
Pikachu

# 短縮オプション
$ poke-lookup ピカチュウ -c
Pikachu

# インタラクティブ選択でも再生
$ poke-lookup フシギ -c
# 選択確定時に鳴き声が鳴ります
```

音声は [PokeAPI/cries](https://github.com/PokeAPI/cries) から取得し、初回のみダウンロードしてローカルにキャッシュします。取得と再生はバックグラウンドで行うため、英名の出力やスプライト表示は待たされません。ただし音を最後まで鳴らすため、コマンド自体は再生（約1秒）の終了を待ちます。

取得は3秒でタイムアウトし、音声デバイスが無い環境と同じく黙ってスキップされます。いずれの場合も標準出力は変わらないので、パイプライン中で使っても影響ありません。

**注意:** 鳴き声機能はデフォルトで有効です。最小構成（`--no-default-features`）でビルドした場合のみ無効になります。Linux では ALSA の開発ヘッダ（`libasound2-dev`）が必要です。

### データ更新

```bash
# 最新のデータを取得
poke-lookup update

# SHA256チェックサム検証付き
poke-lookup update --verify-sha256 <HASH>

# 検証のみ（実際の更新はしない）
poke-lookup update --dry-run
```

### 他ツールとの連携

```bash
# クリップボードにコピー
poke-lookup ピカチュウ | pbcopy

# PokéAPI と連携してポケモンの詳細情報を取得
poke-lookup ピカチュウ | xargs -I {} curl -s "https://pokeapi.co/api/v2/pokemon/{}"

# Pokemiro（Miroツール）での使用
# 1. poke-lookup でポケモン名を取得
# 2. 出力された英名をPokemiroに入力してポケモン画像を表示
```

### アンインストール

```bash
cargo uninstall poke-lookup
```

## データファイルの場所

データファイルは XDG 規約に従って以下の場所に保存されます：

- **Linux**: `~/.local/share/poke-lookup/names.json`
- **macOS**: `~/Library/Application Support/poke-lookup/names.json`
- **Windows**: `C:\Users\{user}\AppData\Roaming\poke-lookup\names.json`

## 終了コード

- `0`: 成功（英名を標準出力に出力）
- `1`: 一般的なエラー
- `2`: 候補が見つからなかった
- `130`: ユーザーによるキャンセル（Ctrl+C 相当）

## データ更新について

- GitHub Actions により毎月1日に自動更新
- PokéAPI から全ポケモン種（1025+）と、そのフォルム（アローラのすがた・メガシンカなど）のデータを取得
- GitHub Releases で配布（SHA256 チェックサム付き）

## トラブルシューティング

### "Data file not found" エラーが出る場合

初回セットアップを実行してください：

```bash
poke-lookup update
```

### インタラクティブ選択が動作しない場合

操作方法：
- `↑` / `↓` または `Ctrl+P` / `Ctrl+N`: 上下移動
- `Enter`: 選択確定
- `Ctrl+C` / `Esc`: キャンセル

### スプライト表示されない場合

以下を確認してください：

1. **スプライト機能の有効化**
   ```bash
   # スプライト機能付きでビルドされているか確認
   poke-lookup --help | grep show-sprite
   ```

2. **対応ターミナルの使用**
   - iTerm2、Kitty、WezTerm などの画像表示対応ターミナルを使用

3. **初回データダウンロード**
   ```bash
   poke-lookup update
   ```

4. **最小構成でビルドした場合**
   ```bash
   # デフォルト（全機能付き）で再ビルド
   cargo install --path . --force
   ```

## 開発

### テスト実行

```bash
# 全機能のテスト（デフォルト）
cargo test

# 最小構成のテスト
cargo test --no-default-features
```

### データ取得スクリプト（CI/CD 用）

```bash
python3 .github/scripts/fetch-pokemon-data.py
```

## ライセンス

MIT

## 貢献

Issue や Pull Request を歓迎します。

## 関連プロジェクト

- [PokéAPI](https://pokeapi.co/) - ポケモンデータの提供元
- [Pokémiro](https://miro.com/marketplace/pokemiro/) - Miroボードにポケモンを追加できる連携可能なツール
