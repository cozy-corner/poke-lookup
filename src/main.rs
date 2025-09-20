mod data;
mod interactive;
mod models;
mod search;
#[cfg(feature = "sprites")]
mod sprite;
mod update;

use anyhow::Result;
use clap::{Parser, Subcommand};
use interactive::InteractiveSelector;
use search::SearchService;
use std::path::PathBuf;
use std::process;
use update::UpdateService;

#[derive(Parser)]
#[command(
    name = "poke-lookup",
    version = "0.1.0",
    about = "日本語名からポケモンの英名を取得するツール (PokéAPI準拠)",
    long_about = "日本語名（カタカナ）を入力すると PokéAPI 準拠の英名を返すCLI。\n出力結果は Pokemiro でそのまま利用できます。"
)]
struct Cli {
    /// ポケモンの日本語名（カタカナ、種レベル）
    #[arg(help = "ポケモンの日本語名（カタカナ、種レベル）")]
    japanese_name: Option<String>,

    /// names.json の明示パス
    #[arg(long = "dict", value_name = "PATH", help = "names.json の明示パス")]
    dict_path: Option<PathBuf>,

    /// スプライト画像を表示
    #[arg(long = "show-sprite", short = 's', help = "スプライト画像を表示")]
    show_sprite: bool,

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// names.json を更新（既定はCI配布を取得）
    Update {
        /// PokéAPI を直接クロールして生成（通常は不要）
        #[arg(long, help = "PokéAPI を直接クロールして生成（通常は不要）")]
        online: bool,

        /// CI配布のURLを上書き
        #[arg(long = "source", value_name = "URL", help = "CI配布のURLを上書き")]
        source_url: Option<String>,

        /// 取得ファイルの検証
        #[arg(
            long = "verify-sha256",
            value_name = "HEX",
            help = "取得ファイルの検証"
        )]
        verify_sha256: Option<String>,

        /// 置換せず検証のみ
        #[arg(long, help = "置換せず検証のみ")]
        dry_run: bool,
    },
}

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

fn run() -> Result<i32> {
    let cli = Cli::parse();

    match cli.command {
        Some(Commands::Update {
            online,
            source_url,
            verify_sha256,
            dry_run,
        }) => handle_update(cli.dict_path, online, source_url, verify_sha256, dry_run),
        None => {
            // 検索機能
            if let Some(japanese_name) = cli.japanese_name {
                search_pokemon(&japanese_name, cli.dict_path, cli.show_sprite)
            } else {
                // 引数なしの場合、全候補からインタラクティブ選択
                search_interactive_all(cli.dict_path, cli.show_sprite)
            }
        }
    }
}

fn search_pokemon(
    japanese_name: &str,
    dict_path: Option<PathBuf>,
    #[allow(unused_variables)] show_sprite: bool,
) -> Result<i32> {
    // SearchServiceを初期化
    let search_service = if let Some(path) = dict_path {
        SearchService::with_path(path)?
    } else {
        SearchService::new()?
    };

    // インタラクティブセレクターを作成
    let selector = InteractiveSelector::new(search_service.clone());

    // 検索実行
    match selector.select_interactive(japanese_name)? {
        Some(english_name) => {
            // 成功: 英名を標準出力
            println!("{}", english_name);

            // スプライト表示
            #[cfg(feature = "sprites")]
            {
                if show_sprite {
                    display_sprite_for_pokemon(&english_name, &search_service)?;
                }
            }

            Ok(0)
        }
        None => {
            // 候補なし
            eprintln!("候補が見つかりませんでした: {}", japanese_name);
            Ok(2)
        }
    }
}

fn search_interactive_all(
    dict_path: Option<PathBuf>,
    #[allow(unused_variables)] show_sprite: bool,
) -> Result<i32> {
    // SearchServiceを初期化
    let search_service = if let Some(path) = dict_path {
        SearchService::with_path(path)?
    } else {
        SearchService::new()?
    };

    // インタラクティブセレクターを作成
    let selector = InteractiveSelector::new(search_service.clone());

    // 全候補から選択
    match selector.select_from_all()? {
        Some(english_name) => {
            // 成功: 英名を標準出力
            println!("{}", english_name);

            // スプライト表示
            #[cfg(feature = "sprites")]
            {
                if show_sprite {
                    display_sprite_for_pokemon(&english_name, &search_service)?;
                }
            }

            Ok(0)
        }
        None => {
            // ユーザーキャンセル
            Ok(130)
        }
    }
}

fn handle_update(
    dict_path: Option<PathBuf>,
    online: bool,
    source_url: Option<String>,
    verify_sha256: Option<String>,
    dry_run: bool,
) -> Result<i32> {
    if online {
        eprintln!("Online update (PokéAPI crawling) is not yet implemented");
        return Ok(1);
    }

    // UpdateServiceを初期化
    let update_service = if let Some(path) = dict_path {
        UpdateService::with_path(path)?
    } else {
        UpdateService::new()?
    };

    // 更新実行
    match update_service.update(source_url, verify_sha256, dry_run) {
        Ok(()) => Ok(0),
        Err(e) => {
            eprintln!("Update failed: {:?}", e);
            Ok(1)
        }
    }
}

#[cfg(feature = "sprites")]
fn display_sprite_for_pokemon(english_name: &str, _search_service: &SearchService) -> Result<()> {
    use crate::sprite::SpriteService;

    let sprite_service = SpriteService::new()?;
    sprite_service.display_sprite_for_pokemon(english_name)?;

    Ok(())
}

#[cfg(not(feature = "sprites"))]
#[allow(dead_code)]
fn display_sprite_for_pokemon(_english_name: &str, _search_service: &SearchService) -> Result<()> {
    eprintln!("スプライト機能は無効です。--features sprites でビルドしてください。");
    Ok(())
}
