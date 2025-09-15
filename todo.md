# 開発計画（機能単位）

## フェーズ1: 基盤構築
- [x] **プロジェクトセットアップ**
  - Cargo.toml作成、依存関係定義
  - 基本的なディレクトリ構造

- [x] **データモデル定義**
  - `names.json`のserde構造体
  - schema_version、generated_at、entries管理

- [x] **データアクセス層**
  - XDGディレクトリ対応のパス解決
  - JSONファイル読み込み、HashMap変換

## フェーズ2: コア機能
- [x] **基本検索機能**
  - 完全一致検索（O(1)）
  - 英名の標準出力

- [x] **インタラクティブ選択**
  - skim統合
  - インクリメンタル検索
  - Emacsキーバインド対応

- [x] **CLIインターフェース**
  - clap設定
  - コマンド引数処理
  - ヘルプ表示

## フェーズ3: 更新機能
- [x] **基本更新機能**
  - HTTPSダウンロード（reqwest）
  - デフォルトURL管理

- [x] **セキュリティ機能**
  - SHA256検証
  - schema_version/countチェック

## フェーズ4: 配布準備
- [x] **CI/CD設定（Phase 1: 基盤構築）**
  - GitHub Actions月次データ更新ワークフロー作成
  - サンプルnames.json生成スクリプト
  - SHA256ハッシュ計算とGitHub Release自動作成
  - poke-lookup updateコマンドの動作確認

- [ ] **CI/CD設定（Phase 2: 実データ統合）**
  - [x] **データ取得スクリプト作成**（.github/scripts/fetch-pokemon-data.py）
    - PokéAPIから全ポケモン種数を動的取得（1025件）
    - 各ポケモンから ja-Hrkt と en の名前ペアを抽出
    - names.json 形式で出力（SHA256ハッシュ付き）
  - [ ] **GitHub Actionsワークフロー更新**（update-data.yml）
    - サンプルデータ生成をスクリプト実行に置換
    - timeout延長とログ出力改善
  - [ ] **データ品質・エラーハンドリング強化**
    - validation機能追加（重複除去、空エントリチェック等）
    - エラー処理の改善

- [ ] **バイナリ配布（オプション）**
  - クロスコンパイル設定（Linux/macOS/Windows）
  - GitHub Releaseでのバイナリ配布

- [ ] **初回セットアップ改善**
  - 初回実行時の自動データダウンロード機能
  - データファイル存在チェック
  - ユーザーフレンドリーな初期化メッセージ

- [ ] **ドキュメント**
  - README作成
  - インストール・使用方法
