# フェーズ2: CLIインターフェースの学び

## 1. clap を使ったCLI設計

### derive マクロの活用
```rust
#[derive(Parser)]
#[command(
    name = "poke-lookup",
    version = "0.1.0",
    about = "日本語名からポケモンの英名を取得するツール (PokéAPI準拠)"
)]
struct Cli {
    #[arg(help = "ポケモンの日本語名（カタカナ、種レベル）")]
    japanese_name: Option<String>,

    #[arg(long = "dict", value_name = "PATH")]
    dict_path: Option<PathBuf>,

    #[command(subcommand)]
    command: Option<Commands>,
}
```

### サブコマンドの定義
```rust
#[derive(Subcommand)]
enum Commands {
    Update {
        #[arg(long)]
        online: bool,

        #[arg(long = "source", value_name = "URL")]
        source_url: Option<String>,
    },
}
```

## 2. 終了コードの設計

### UNIX慣習に従った終了コード
- **0**: 成功
- **1**: 一般的なエラー
- **2**: 候補なし（独自定義）
- **130**: ユーザーキャンセル（Ctrl+C相当）

### 実装パターン
```rust
fn main() {
    let result = run();
    match result {
        Ok(exit_code) => process::exit(exit_code),
        Err(e) => {
            eprintln!("Error: {:?}", e);
            process::exit(1);
        }
    }
}
```

## 3. UX設計の重要性

### 自動確定の課題
**当初の要件**: 「候補が1件に絞れた場合は自動確定」

**実装で発見した問題**:
- インタラクティブ中の突然終了は不自然
- ユーザーの操作フローを阻害
- 意図しない確定のリスク

**最終的な解決策**:
- 完全一致のみ自動確定
- 部分一致は常にインタラクティブ選択
- ユーザーの明示的確認を重視

### skimの制限理解
```rust
// select1は起動時のみ有効
.select1(true)  // インタラクティブ中の絞り込みには適用されない
```

## 4. 標準出力の設計

### パイプライン対応
```bash
poke-lookup ピカチュウ | pbcopy  # 英名のみがコピーされる
poke-lookup フシギダネ | other-command  # 連携可能
```

### 出力の分離
- **標準出力**: 英名のみ（他ツールとの連携用）
- **標準エラー出力**: エラーメッセージ、操作ガイド

## 5. 実装の一貫性

### 設計原則
1. **完全一致**: 即座に結果出力
2. **部分一致**: インタラクティブ選択
3. **候補なし**: エラーメッセージ + 適切な終了コード

### コードの整理
```rust
// 不要なインポートの削除
use anyhow::Result;  // Context は未使用なので削除

// dead_code警告の適切な管理
#[allow(dead_code)]  // 更新機能で使用予定
pub fn entry_count(&self) -> usize { ... }
```

## 6. テスト手法

### クロスプラットフォーム対応
- **Linux**: `~/.local/share/poke-lookup/`
- **macOS**: `~/Library/Application Support/poke-lookup/`
- **Windows**: `C:\Users\{user}\AppData\Roaming\poke-lookup\`

### テストデータの活用
```json
{
  "schema_version": 1,
  "generated_at": "2025-01-14T10:00:00Z",
  "count": 5,
  "entries": [
    { "ja": "ピカチュウ", "en": "Pikachu" },
    { "ja": "フシギダネ", "en": "Bulbasaur" }
  ]
}
```

## 7. エラーハンドリング

### 分かりやすいエラーメッセージ
```rust
if !self.data_path.exists() {
    return Err(anyhow::anyhow!(
        "Data file not found: {}. Run 'poke-lookup update' to download the data file.",
        self.data_path.display()
    ));
}
```

## 8. 仕様の柔軟性

### 実装による仕様の見直し
- 理論的に良さそうな機能も、実装すると問題が見つかる
- ユーザビリティを最優先に仕様を調整
- 技術的制約（skimの機能）も考慮

## 9. Rustらしいエラーハンドリング（他言語との比較）

### エラー伝播演算子 `?` の威力

**Rust の実装:**
```rust
fn search_pokemon(japanese_name: &str) -> Result<i32> {
    let search_service = SearchService::new()?;  // エラーなら即return
    let selector = InteractiveSelector::new(search_service);

    match selector.select_interactive(japanese_name)? {
        Some(english_name) => {
            println!("{}", english_name);
            Ok(0)
        }
        None => {
            eprintln!("候補が見つかりませんでした");
            Ok(2)
        }
    }
}
```

**Kotlin（Result型）での同等コード:**
```kotlin
fun searchPokemon(name: String): Result<Int> {
    return SearchService.new()
        .mapCatching { service ->
            InteractiveSelector(service)
        }
        .mapCatching { selector ->
            selector.selectInteractive(name)
        }
        .map { result ->
            when (result) {
                null -> {
                    System.err.println("候補が見つかりませんでした")
                    2
                }
                else -> {
                    println(result)
                    0
                }
            }
        }
}
```

**Kotlin Arrow（Either型）での同等コード:**
```kotlin
import arrow.core.*
import arrow.core.computations.either

// Either型を使った実装
suspend fun searchPokemon(name: String): Either<AppError, Int> = either {
    val searchService = SearchService.new().bind()  // bindで早期リターン可能
    val selector = InteractiveSelector(searchService)

    val result = selector.selectInteractive(name).bind()

    when (result) {
        null -> {
            System.err.println("候補が見つかりませんでした")
            2
        }
        else -> {
            println(result)
            0
        }
    }
}

// より関数型スタイル
fun searchPokemonFP(name: String): Either<AppError, Int> =
    SearchService.new()
        .flatMap { service ->
            InteractiveSelector(service).selectInteractive(name)
        }
        .map { result ->
            result?.let {
                println(it)
                0
            } ?: run {
                System.err.println("候補が見つかりませんでした")
                2
            }
        }
```

### 言語比較表

| 特徴 | Rust `?` | Kotlin Result | Kotlin Arrow Either |
|------|----------|---------------|---------------------|
| **構文の簡潔さ** | ◎ 非常に簡潔 | ○ チェーンで書ける | ○ bind で簡潔化可能 |
| **型安全性** | ◎ コンパイル時保証 | ○ 型で表現 | ◎ 型で完全表現 |
| **エラーの明示性** | ◎ Result型で明示 | ○ Result型 | ◎ Either型で左右明確 |
| **早期リターン** | ◎ `?`で自動 | × 難しい | ○ bind で可能 |
| **標準機能** | ◎ 言語組み込み | ◎ 標準ライブラリ | × 外部ライブラリ |
| **関数型スタイル** | △ 必要に応じて | ○ 可能 | ◎ 完全サポート |
| **学習コスト** | ○ シンプル | ○ 直感的 | △ 関数型の知識必要 |

### Rustの`?`演算子の利点

1. **簡潔性と可読性の両立**
   - エラーハンドリングが1文字
   - 正常系のフローが明確

2. **型安全性の強制**
   - 全てのエラーがResult型で表現される
   - 未処理エラーはコンパイルエラー

3. **早期リターンの自然な表現**
   - ガード節的な書き方が簡単
   - ネストを避けられる

### 学んだこと

- Rustの`?`演算子は**エラーハンドリングのボイラープレートを劇的に削減**
- **型安全性を犠牲にせず簡潔性を実現**している点が優れている
- 他言語（Kotlin等）と比較すると、Rustのエラーハンドリングは言語レベルで最適化されている

## まとめ

フェーズ2では、CLIツールのUX設計の重要性を学びました：

1. **技術仕様だけでなく、実際の使用感を重視**
2. **パイプライン対応などUNIX哲学への準拠**
3. **適切な終了コードとエラーハンドリング**
4. **実装過程での仕様の柔軟な見直し**

これらの学びにより、自然で使いやすいCLIツールの基盤が完成しました。
