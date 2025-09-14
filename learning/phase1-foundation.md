# フェーズ1: 基盤構築の学び

## 1. Rustのプロジェクト構造

### Cargo.tomlのedition
- **edition = "2024"** - 3年ごとにリリースされる言語仕様
- 異なるエディション間でも相互運用可能
- 最新エディションを使うのが一般的

### 依存関係の管理
```toml
[dependencies]
chrono = { version = "0.4", features = ["serde"] }  # features指定が重要

[dev-dependencies]
tempfile = "3.0"  # テスト専用の依存関係
```

## 2. Rustの命名規則

### 避けるべきパターン
- `NamesData` - "Data"は冗長
- `NameInfo` - "Info"も冗長
- 意味を追加しない接尾辞は避ける

### 推奨パターン
- `NameDictionary` - 具体的な責務を表す
- `NameEntry` - シンプルで明確

## 3. 型システムと慣習

### DateTime vs String
```rust
// ❌ 一般的ではない
pub generated_at: String

// ✅ Rust的
pub generated_at: DateTime<Utc>
```
- 型安全性を重視
- 日時の計算・比較が可能

### Result型の活用
```rust
// anyhowによる省略形
pub fn load_dictionary(&self) -> Result<NameDictionary>
// 実際は: Result<NameDictionary, anyhow::Error>
```

### Unit型 `()`
```rust
pub fn ensure_data_dir(&self) -> Result<()>
// 値を返さないが成功/失敗を表現
```

## 4. テストの配置

### 単体テストは同じファイル内
```rust
// src/models.rs
pub struct NameDictionary { ... }

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_something() { ... }
}
```
- プライベート関数もテスト可能
- `#[cfg(test)]`でビルド時は除外

## 5. エラーハンドリング

### anyhowの活用
```rust
use anyhow::{Context, Result};

// コンテキスト付きエラー
fs::read_to_string(&path)
    .with_context(|| format!("Failed to read: {}", path.display()))?;

// Stringエラーの変換
dictionary.validate()
    .map_err(|e| anyhow::anyhow!("Validation failed: {}", e))?;
```

## 6. XDGディレクトリ規約

### クロスプラットフォーム対応
- **Linux**: `~/.local/share/app/`
- **macOS**: `~/Library/Application Support/app/`
- **Windows**: `C:\Users\{user}\AppData\Roaming\app\`

### ProjectDirs使用時の注意
```rust
// 空文字列でも動作するが、フォールバックを用意
ProjectDirs::from("", "", "poke-lookup")
    .or_else(|| ProjectDirs::from("dev", "poke-lookup", "poke-lookup"))
```

## 7. テスト設計

### 成功と失敗の両方をテスト
```rust
#[test]
fn test_ensure_data_dir() { ... }  // 成功ケース

#[test]
#[cfg(unix)]  // OS固有のテスト
fn test_ensure_data_dir_permission_error() { ... }  // 失敗ケース
```

### tempfileクレートの活用
```rust
use tempfile::tempdir;

let temp_dir = tempdir().unwrap();
// テスト終了時に自動削除
```

## 8. Rust哲学

### YAGNI (You Aren't Gonna Need It)
- 過度な防御的コードは避ける
- 実際に問題が発生してから対応

### DRY原則
- 型推論を活用
- 省略可能な部分は省略

### エラーは明示的に
- panicよりResult
- エラー情報を失わない設計

## まとめ

フェーズ1では、Rustらしいコード設計の基礎を学びました：
- 型安全性を重視した設計
- 適切な命名規則
- テストファーストな開発
- クロスプラットフォーム対応
- エラーハンドリングのベストプラクティス

これらの基盤により、堅牢で保守しやすいCLIツールの土台が完成しました。