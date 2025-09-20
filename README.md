# poke-lookup

日本語のポケモン名から PokéAPI 準拠の英名を取得する CLI ツール

## 概要

`poke-lookup` は、カタカナのポケモン名を入力すると対応する英名を返すコマンドラインツールです。
[Pokémiro](https://miro.com/marketplace/pokemiro/) などの他ツールと連携して使用できます。

## 特徴

- 🔍 高速な検索（HashMap による O(1) アクセス）
- 🎯 完全一致で即座に結果を返す
- 📝 部分一致時はインタラクティブ選択（skim 使用）
- 🖼️ ターミナル内スプライト表示（オプション機能）
- 🔄 月次自動データ更新（GitHub Actions）
- 🔒 SHA256 によるデータ整合性チェック
- 🌐 PokéAPI 準拠のデータ

## インストール

### 前提条件

- Rust 1.70以上
- Git（クローン用）
- インターネット接続（初回セットアップ時のデータダウンロード用）

### インストール手順

```bash
git clone https://github.com/cozy-corner/poke-lookup.git
cd poke-lookup
cargo install --path .
```

これにより `poke-lookup` コマンドがどこからでも実行可能になります。

### スプライト機能付きインストール

ターミナル内でポケモンのスプライト画像を表示したい場合：

```bash
cargo install --path . --features sprites
```

### 手動ビルド（開発用）

```bash
# 基本機能のみ
cargo build --release

# スプライト機能付き
cargo build --release --features sprites
```

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

### スプライト表示（オプション機能）

スプライト機能付きでビルドした場合、ポケモンの画像をターミナル内に表示できます：

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

**注意:** スプライト機能は `--features sprites` でビルドした場合のみ利用可能です。

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
- PokéAPI から全ポケモン種（1025+）のデータを取得
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

4. **フィーチャー無効でビルドした場合**
   ```bash
   # スプライト機能付きで再ビルド
   cargo install --path . --features sprites --force
   ```

## 開発

### テスト実行

```bash
# 基本機能のテスト
cargo test

# 全機能のテスト（スプライト機能含む）
cargo test --all-features
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