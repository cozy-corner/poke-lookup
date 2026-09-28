#[cfg(feature = "sprites")]
use anyhow::{Context, Result};
#[cfg(feature = "sprites")]
use reqwest::blocking::Client;
#[cfg(feature = "sprites")]
use serde::Deserialize;
#[cfg(feature = "sprites")]
use std::collections::{HashMap, HashSet};

/// /pokemon/{id} は数KB程度。返らないなら諦めて情報表示を省く
#[cfg(feature = "sprites")]
const INFO_FETCH_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(3);

/// 種族値スラッグ → 日本語ラベル。ゲージを揃えるため表示幅8桁に padding 済み
/// （日本語4文字＝全角で8桁、HP は半角2文字＋空白6）
#[cfg(feature = "sprites")]
fn stat_ja(slug: &str) -> Option<&'static str> {
    Some(match slug {
        "hp" => "HP      ",
        "attack" => "こうげき",
        "defense" => "ぼうぎょ",
        "special-attack" => "とくこう",
        "special-defense" => "とくぼう",
        "speed" => "すばやさ",
        _ => return None,
    })
}

#[cfg(feature = "sprites")]
#[derive(Debug, Deserialize)]
struct PokemonResponse {
    types: Vec<TypeSlot>,
    stats: Vec<StatSlot>,
    // フォルムでも正しい種を辿れるよう species の URL を使う
    #[serde(default)]
    species: Option<UrlRef>,
}

#[cfg(feature = "sprites")]
#[derive(Debug, Deserialize)]
struct UrlRef {
    url: String,
}

#[cfg(feature = "sprites")]
#[derive(Debug, Deserialize)]
struct SpeciesResponse {
    flavor_text_entries: Vec<FlavorEntry>,
}

#[cfg(feature = "sprites")]
#[derive(Debug, Deserialize)]
struct FlavorEntry {
    flavor_text: String,
    language: NamedRef,
}

/// 図鑑説明文の改行/フォームフィードを除去（日本語は語間の空白が無いため詰める）
#[cfg(feature = "sprites")]
fn clean_flavor(text: &str) -> String {
    text.replace(['\n', '\r', '\u{c}'], "")
}

/// `.../pokemon-species/25/` 形式の末尾から species ID を取り出す
#[cfg(feature = "sprites")]
fn id_from_url(url: &str) -> Option<u32> {
    url.trim_end_matches('/').rsplit('/').next()?.parse().ok()
}

#[cfg(feature = "sprites")]
#[derive(Debug, Deserialize)]
struct TypeSlot {
    #[serde(rename = "type")]
    type_ref: NamedRef,
}

#[cfg(feature = "sprites")]
#[derive(Debug, Deserialize)]
struct StatSlot {
    base_stat: u16,
    stat: NamedRef,
}

#[cfg(feature = "sprites")]
#[derive(Debug, Deserialize)]
struct NamedRef {
    name: String,
}

/// /type/{name} の応答。防御側から見た相性だけを使う
#[cfg(feature = "sprites")]
#[derive(Debug, Deserialize)]
struct TypeResponse {
    damage_relations: DamageRelations,
}

#[cfg(feature = "sprites")]
#[derive(Debug, Deserialize)]
struct DamageRelations {
    #[serde(default)]
    double_damage_from: Vec<NamedRef>,
    #[serde(default)]
    half_damage_from: Vec<NamedRef>,
    #[serde(default)]
    no_damage_from: Vec<NamedRef>,
}

/// タイプの英語スラッグ → チップ背景色の SGR プレフィックス（背景色＋文字色）。
/// 色は公式タイプカラーに近い256色。暗い背景のタイプだけ文字を白にする
#[cfg(feature = "sprites")]
fn type_color(slug: &str) -> &'static str {
    // "\x1b[48;5;{bg}m\x1b[38;5;{fg}m" 形式。fg 16=黒, 231=白
    match slug {
        "normal" => "\x1b[48;5;144m\x1b[38;5;16m",
        "fire" => "\x1b[48;5;208m\x1b[38;5;16m",
        "water" => "\x1b[48;5;75m\x1b[38;5;16m",
        "electric" => "\x1b[48;5;220m\x1b[38;5;16m",
        "grass" => "\x1b[48;5;113m\x1b[38;5;16m",
        "ice" => "\x1b[48;5;159m\x1b[38;5;16m",
        "fighting" => "\x1b[48;5;131m\x1b[38;5;231m",
        "poison" => "\x1b[48;5;133m\x1b[38;5;231m",
        "ground" => "\x1b[48;5;179m\x1b[38;5;16m",
        "flying" => "\x1b[48;5;141m\x1b[38;5;16m",
        "psychic" => "\x1b[48;5;205m\x1b[38;5;16m",
        "bug" => "\x1b[48;5;142m\x1b[38;5;16m",
        "rock" => "\x1b[48;5;137m\x1b[38;5;16m",
        "ghost" => "\x1b[48;5;60m\x1b[38;5;231m",
        "dragon" => "\x1b[48;5;62m\x1b[38;5;231m",
        "dark" => "\x1b[48;5;238m\x1b[38;5;231m",
        "steel" => "\x1b[48;5;146m\x1b[38;5;16m",
        "fairy" => "\x1b[48;5;218m\x1b[38;5;16m",
        _ => "",
    }
}

/// 表示用の1件のタイプ
#[cfg(feature = "sprites")]
pub struct PokemonType {
    pub ja: String,
    /// チップ背景色の SGR プレフィックス。未知タイプは空（色なし）
    pub color: &'static str,
}

/// 表示用の1件の弱点。防御側から見た相性で、弱点だけを扱う
#[cfg(feature = "sprites")]
pub struct Weakness {
    pub ja: String,
    /// 受けるダメージ倍率。弱点のみ扱うので 2 か 4
    pub multiplier: u8,
    /// チップ背景色の SGR プレフィックス。未知タイプは空（色なし）
    pub color: &'static str,
}

/// 表示用に整形済みの1件の種族値
#[cfg(feature = "sprites")]
pub struct StatEntry {
    /// 表示幅8桁に揃えた日本語ラベル
    pub label: &'static str,
    pub value: u16,
}

/// 画像の下に出すポケモン情報
#[cfg(feature = "sprites")]
pub struct PokemonInfo {
    pub types: Vec<PokemonType>,
    /// 弱点（×2 / ×4 のみ）。取得できなければ空
    pub weaknesses: Vec<Weakness>,
    pub stats: Vec<StatEntry>,
    /// 図鑑説明文（日本語）。取得できなければ None
    pub description: Option<String>,
}

/// ポケモンの付加情報（タイプ・種族値）の取得を管理するサービス
#[cfg(feature = "sprites")]
pub struct PokemonInfoService {
    client: Client,
    base_url: String,
    id_map: HashMap<String, u32>,
}

#[cfg(feature = "sprites")]
impl PokemonInfoService {
    pub fn new() -> Result<Self> {
        use crate::data::DataLoader;

        let client = Client::builder()
            .user_agent(concat!("poke-lookup/", env!("CARGO_PKG_VERSION")))
            .timeout(INFO_FETCH_TIMEOUT)
            .build()
            .context("Failed to create HTTP client")?;

        let loader = DataLoader::new()?;
        let dictionary = loader.load_dictionary()?;
        let id_map = dictionary
            .entries
            .iter()
            .filter_map(|entry| entry.id.map(|id| (entry.en.clone(), id)))
            .collect();

        Ok(Self {
            client,
            base_url: "https://pokeapi.co/api/v2".to_string(),
            id_map,
        })
    }

    pub fn get_pokemon_id(&self, english_name: &str) -> Option<u32> {
        self.id_map.get(english_name).copied()
    }

    /// 英名からタイプと種族値を1リクエストで取得。取得できなければ None
    pub fn fetch(&self, english_name: &str) -> Option<PokemonInfo> {
        let id = self.get_pokemon_id(english_name)?;

        let url = format!("{}/pokemon/{}", self.base_url, id);
        let response = match self.client.get(&url).send() {
            Ok(r) if r.status().is_success() => r,
            _ => return None,
        };

        let body = response.json::<PokemonResponse>().ok()?;

        // 未知スラッグはそのまま出してフォールバック
        let types = body
            .types
            .iter()
            .map(|slot| PokemonType {
                ja: crate::pokemon_type::type_ja(&slot.type_ref.name)
                    .map(str::to_string)
                    .unwrap_or_else(|| slot.type_ref.name.clone()),
                color: type_color(&slot.type_ref.name),
            })
            .collect();

        // ラベルを解決できた種族値だけ（順序はAPIの HP→こうげき→…→すばやさ）
        let stats = body
            .stats
            .iter()
            .filter_map(|slot| {
                stat_ja(&slot.stat.name).map(|label| StatEntry {
                    label,
                    value: slot.base_stat,
                })
            })
            .collect();

        // フォルムは form id から species を辿れない（/pokemon-species/{form_id} は
        // 404）。応答の species URL から species id を取り出し、リクエスト自体は
        // base_url に根ざして組み立てる（テスト可能・リモートURL追従を避ける）
        let description = body
            .species
            .as_ref()
            .and_then(|sp| id_from_url(&sp.url))
            .and_then(|species_id| self.fetch_description(species_id));

        // 弱点は自分のタイプごとに /type/{name} を引いて掛け合わせる
        let own_type_slugs: Vec<String> = body
            .types
            .iter()
            .map(|slot| slot.type_ref.name.clone())
            .collect();
        let weaknesses = self.fetch_weaknesses(&own_type_slugs);

        Some(PokemonInfo {
            types,
            weaknesses,
            stats,
            description,
        })
    }

    /// species ID から日本語の図鑑説明文を1件取得。失敗時は None
    fn fetch_description(&self, species_id: u32) -> Option<String> {
        let url = format!("{}/pokemon-species/{}", self.base_url, species_id);
        let response = self.client.get(&url).send().ok()?;
        if !response.status().is_success() {
            return None;
        }
        let species = response.json::<SpeciesResponse>().ok()?;
        // 漢字かな交じり(ja)を優先。無ければ かな(ja-Hrkt) にフォールバック
        let entry = species
            .flavor_text_entries
            .iter()
            .find(|e| e.language.name == "ja")
            .or_else(|| {
                species
                    .flavor_text_entries
                    .iter()
                    .find(|e| e.language.name == "ja-Hrkt")
            })?;
        Some(clean_flavor(&entry.flavor_text))
    }

    /// 自分のタイプごとに /type/{name} を引き、攻撃タイプごとに倍率を掛け合わせて
    /// 弱点（×2 / ×4）だけを返す。複合タイプの相性は和集合ではなく積なので、
    /// 耐性(×0.5)と無効(×0)も必ず読む（無効は他のタイプの弱点を打ち消す）
    fn fetch_weaknesses(&self, own_type_slugs: &[String]) -> Vec<Weakness> {
        // 攻撃タイプのスラッグ -> 2 の指数（×2 で +1、×0.5 で -1、等倍は 0）
        let mut exponent: HashMap<String, i32> = HashMap::new();
        // 1 つでも無効(×0)なら、他のタイプが弱点でも合計は 0 のまま
        let mut immune: HashSet<String> = HashSet::new();

        for slug in own_type_slugs {
            let Some(relations) = self.fetch_damage_relations(slug) else {
                continue;
            };
            for target in &relations.double_damage_from {
                *exponent.entry(target.name.clone()).or_insert(0) += 1;
            }
            for target in &relations.half_damage_from {
                *exponent.entry(target.name.clone()).or_insert(0) -= 1;
            }
            for target in &relations.no_damage_from {
                immune.insert(target.name.clone());
            }
        }

        let mut weaknesses: Vec<(String, u8)> = exponent
            .into_iter()
            .filter(|(slug, exp)| *exp >= 1 && !immune.contains(slug))
            // 指数 1 で ×2、2 で ×4。実データは 2 タイプまでだが、
            // 異常値で u8 から溢れないよう u32 で計算してから丸める
            .map(|(slug, exp)| (slug, (1u32 << exp).min(u8::MAX as u32) as u8))
            .collect();

        // 倍率の高い順、同倍率内はスラッグ順（表示を安定させる）
        weaknesses.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));

        weaknesses
            .into_iter()
            .map(|(slug, multiplier)| Weakness {
                // 未知スラッグはそのまま出してフォールバック
                ja: crate::pokemon_type::type_ja(&slug)
                    .map(str::to_string)
                    .unwrap_or_else(|| slug.clone()),
                multiplier,
                color: type_color(&slug),
            })
            .collect()
    }

    /// /type/{slug} の相性を取得。失敗時は None（弱点表示だけ諦める）
    fn fetch_damage_relations(&self, type_slug: &str) -> Option<DamageRelations> {
        let url = format!("{}/type/{}", self.base_url, type_slug);
        let response = self.client.get(&url).send().ok()?;
        if !response.status().is_success() {
            return None;
        }
        response
            .json::<TypeResponse>()
            .ok()
            .map(|t| t.damage_relations)
    }

    #[cfg(test)]
    pub fn for_test(base_url: String, id_map: HashMap<String, u32>) -> Self {
        Self {
            client: Client::new(),
            base_url,
            id_map,
        }
    }
}

// --- 表示整形（ネット不要の純粋関数。スプライト下に印字する文字列を組み立てる） ---

/// 種族値ゲージの棒の長さ（文字数）
#[cfg(feature = "sprites")]
const GAUGE_WIDTH: usize = 20;
/// 種族値ゲージの上限。技術上限は255だが大半が50〜130に収まるため、
/// メリハリ優先で150を満杯とし、超過分はクランプする
#[cfg(feature = "sprites")]
const STAT_MAX: u16 = 150;
/// 塗り部分の色（256色パレットの黄色/ゴールド）
#[cfg(feature = "sprites")]
const GAUGE_FILL: &str = "\x1b[38;5;220m";
/// 空白部分の色。塗りと同系統の暗い黄土色で「同じバーの空き」に見せる
#[cfg(feature = "sprites")]
const GAUGE_EMPTY: &str = "\x1b[38;5;58m";
/// SGR リセット（色指定の打ち消し）
#[cfg(feature = "sprites")]
const SGR_RESET: &str = "\x1b[0m";
/// ×4（4倍弱点）のチップに添える警告マーク。太字(SGR 1)は日本語フォントに
/// 太字が無い端末では描画されないため、フォント依存の無い文字で強調する
#[cfg(feature = "sprites")]
const QUAD_MARKER: &str = "⚠";

/// 名前（と取れれば図鑑番号）の見出し行。名前は必ず出す。
/// info サービス初期化失敗などで id が無くても、選択したポケモンが分かるように
#[cfg(feature = "sprites")]
pub fn format_header(id: Option<u32>, japanese: Option<&str>, english: &str) -> String {
    let name = match japanese {
        Some(ja) => format!("{} ({})", ja, english),
        None => english.to_string(),
    };
    match id {
        Some(id) => format!("\nNo.{}  {}\n", id, name),
        None => format!("\n{}\n", name),
    }
}

/// タイプ・種族値・説明をまとめた本文。取得できた部分だけを連結する
#[cfg(feature = "sprites")]
pub fn format_body(info: &PokemonInfo) -> String {
    let mut out = String::new();
    out.push_str(&format_types(&info.types));
    out.push_str(&format_weaknesses(&info.weaknesses));
    out.push_str(&format_stats(&info.stats));
    if let Some(ref description) = info.description {
        out.push_str(&format!("\n{}\n", description));
    }
    out
}

/// タイプ公式カラーの背景色チップを横に並べた1行。空なら空文字
#[cfg(feature = "sprites")]
fn format_types(types: &[PokemonType]) -> String {
    if types.is_empty() {
        return String::new();
    }
    let chips: Vec<String> = types
        .iter()
        .map(|t| format!("{} {} {}", t.color, t.ja, SGR_RESET))
        .collect();
    format!("\n{}\n", chips.join(" "))
}

/// 弱点をチップで並べた1行。語ラベルの代わりに `←` で「飛んでくる側」を示し、
/// ×4 だけ警告マークを添える。空なら空文字
#[cfg(feature = "sprites")]
fn format_weaknesses(weaknesses: &[Weakness]) -> String {
    if weaknesses.is_empty() {
        return String::new();
    }
    let chips: Vec<String> = weaknesses
        .iter()
        .map(|w| {
            // ×4（2タイプとも弱点）だけ警告マークを添える
            let label = if w.multiplier >= 4 {
                format!("{}{}", QUAD_MARKER, w.ja)
            } else {
                w.ja.clone()
            };
            format!("{} {} {}", w.color, label, SGR_RESET)
        })
        .collect();
    // チップ自身が前後に空白を持つので、矢印の後にも 1 文字あけておく
    // （あけないと矢印がチップの色ブロックに接して詰まって見える）。
    // 区切りはタイプ行と同じに揃える
    format!("\n ← {}\n", chips.join(" "))
}

/// 種族値ゲージ（各行）と合計。空なら空文字
#[cfg(feature = "sprites")]
fn format_stats(stats: &[StatEntry]) -> String {
    if stats.is_empty() {
        return String::new();
    }
    let mut out = String::from("\n");
    let mut total = 0u32;
    for stat in stats {
        total += stat.value as u32;
        out.push_str(&format_stat_gauge(stat.label, stat.value));
        out.push('\n');
    }
    out.push_str(&format!("ごうけい  {:>3}\n", total));
    out
}

/// `ラベル  値 ████░░░░` 形式の1行を作る。label は表示幅8桁に揃え済みで、
/// 値との間には固定スペースを2つ入れる（3桁の値でもラベルと密着させないため）
#[cfg(feature = "sprites")]
fn format_stat_gauge(label: &str, value: u16) -> String {
    // ここで STAT_MAX にクランプするため filled は必ず GAUGE_WIDTH 以下。
    // この .min(STAT_MAX) を外すと下の GAUGE_WIDTH - filled がアンダーフローする
    let filled = value.min(STAT_MAX) as usize * GAUGE_WIDTH / STAT_MAX as usize;
    format!(
        "{}  {:>3} {}{}{}{}{}",
        label,
        value,
        GAUGE_FILL,
        "█".repeat(filled),
        GAUGE_EMPTY,
        "░".repeat(GAUGE_WIDTH - filled),
        SGR_RESET,
    )
}

#[cfg(test)]
#[cfg(feature = "sprites")]
mod tests {
    use super::*;

    /// SGR エスケープを除去してテキストだけ残す（色・太字の検証と分離するため）
    fn strip_sgr(s: &str) -> String {
        let mut out = String::new();
        let mut chars = s.chars();
        while let Some(c) = chars.next() {
            if c == '\u{1b}' {
                for c2 in chars.by_ref() {
                    if c2 == 'm' {
                        break;
                    }
                }
            } else {
                out.push(c);
            }
        }
        out
    }

    #[test]
    fn test_format_stat_gauge() {
        // 色コードを剥がして棒の中身だけ検証する
        let bare = |label, value| {
            format_stat_gauge(label, value)
                .replace(GAUGE_FILL, "")
                .replace(GAUGE_EMPTY, "")
                .replace(SGR_RESET, "")
        };
        // 0 は空、上限(150)以上は満杯、中間は比例（35/150*20 = 4 マス）
        assert_eq!(bare("HP      ", 0), "HP          0 ░░░░░░░░░░░░░░░░░░░░");
        assert_eq!(bare("すばやさ", 255), "すばやさ  255 ████████████████████");
        assert_eq!(bare("HP      ", 35), "HP         35 ████░░░░░░░░░░░░░░░░");
        assert!(format_stat_gauge("HP      ", 35).contains(GAUGE_FILL));
    }

    #[test]
    fn test_format_header_with_and_without_ja() {
        assert_eq!(
            format_header(Some(6), Some("リザードン"), "Charizard"),
            "\nNo.6  リザードン (Charizard)\n"
        );
        assert_eq!(
            format_header(Some(6), None, "Charizard"),
            "\nNo.6  Charizard\n"
        );
    }

    #[test]
    fn test_format_header_without_id_still_shows_name() {
        assert_eq!(
            format_header(None, Some("リザードン"), "Charizard"),
            "\nリザードン (Charizard)\n"
        );
        assert_eq!(format_header(None, None, "Charizard"), "\nCharizard\n");
    }

    #[test]
    fn test_format_types_empty_is_blank() {
        assert_eq!(format_types(&[]), "");
    }

    #[test]
    fn test_format_weaknesses_empty_is_blank() {
        assert_eq!(format_weaknesses(&[]), "");
    }

    #[test]
    fn test_format_weaknesses_marks_quad_and_hides_multiplier() {
        let weaknesses = vec![
            Weakness {
                ja: "いわ".to_string(),
                multiplier: 4,
                color: type_color("rock"),
            },
            Weakness {
                ja: "みず".to_string(),
                multiplier: 2,
                color: type_color("water"),
            },
        ];
        let out = format_weaknesses(&weaknesses);
        let plain = strip_sgr(&out);

        // ×4 にだけ警告マークが付き、行ラベルは語ではなく `←` で示す。
        // 矢印とチップの間は 1 文字あける（チップ内の余白と合わせて詰まらないように）
        assert_eq!(plain, "\n ←  ⚠いわ   みず \n");

        // 倍率の数字は出さない
        assert!(!plain.chars().any(|c| c.is_ascii_digit()));
        assert!(!plain.contains('×'));
    }

    #[test]
    fn test_fetch_weaknesses_multiplies_across_types() {
        use httpmock::prelude::*;

        let server = MockServer::start();
        server.mock(|when, then| {
            when.method(GET).path("/pokemon/6");
            then.status(200)
                .header("content-type", "application/json")
                .body(
                    r#"{"types":[
                        {"slot":1,"type":{"name":"fire","url":"x"}},
                        {"slot":2,"type":{"name":"flying","url":"x"}}
                    ],"stats":[]}"#,
                );
        });
        server.mock(|when, then| {
            when.method(GET).path("/type/fire");
            then.status(200)
                .header("content-type", "application/json")
                .body(
                    r#"{"damage_relations":{
                        "double_damage_from":[{"name":"ground"},{"name":"rock"},{"name":"water"}],
                        "half_damage_from":[{"name":"bug"},{"name":"steel"},{"name":"fire"},{"name":"grass"},{"name":"ice"},{"name":"fairy"}],
                        "no_damage_from":[]
                    }}"#,
                );
        });
        server.mock(|when, then| {
            when.method(GET).path("/type/flying");
            then.status(200)
                .header("content-type", "application/json")
                .body(
                    r#"{"damage_relations":{
                        "double_damage_from":[{"name":"rock"},{"name":"electric"},{"name":"ice"}],
                        "half_damage_from":[{"name":"fighting"},{"name":"bug"},{"name":"grass"}],
                        "no_damage_from":[{"name":"ground"}]
                    }}"#,
                );
        });

        let mut id_map = HashMap::new();
        id_map.insert("Charizard".to_string(), 6);
        let service = PokemonInfoService::for_test(server.url(""), id_map);

        let info = service.fetch("Charizard").expect("should fetch");
        let got: Vec<(String, u8)> = info
            .weaknesses
            .iter()
            .map(|w| (w.ja.clone(), w.multiplier))
            .collect();

        // リザードン(fire/flying): いわ は 2×2 で ×4、でんき・みず は ×2
        assert_eq!(
            got,
            vec![
                ("いわ".to_string(), 4),
                ("でんき".to_string(), 2),
                ("みず".to_string(), 2),
            ]
        );
        // じめんは flying の無効(×0)が fire の ×2 を打ち消す
        assert!(!got.iter().any(|(ja, _)| ja == "じめん"));
        // こおりは fire の耐性(×0.5)と flying の ×2 が相殺して等倍
        assert!(!got.iter().any(|(ja, _)| ja == "こおり"));
    }

    #[test]
    fn test_fetch_weaknesses_stay_empty_when_type_lookup_fails() {
        use httpmock::prelude::*;

        let server = MockServer::start();
        // /type/electric はモックしない（404 になる）
        server.mock(|when, then| {
            when.method(GET).path("/pokemon/25");
            then.status(200)
                .header("content-type", "application/json")
                .body(
                    r#"{"types":[{"slot":1,"type":{"name":"electric","url":"x"}}],
                        "stats":[{"base_stat":35,"stat":{"name":"hp"}}]}"#,
                );
        });

        let mut id_map = HashMap::new();
        id_map.insert("Pikachu".to_string(), 25);
        let service = PokemonInfoService::for_test(server.url(""), id_map);

        let info = service.fetch("Pikachu").expect("should fetch");
        assert!(info.weaknesses.is_empty());
        // 弱点が取れなくても、タイプと種族値は残る
        assert_eq!(info.types.len(), 1);
        assert_eq!(info.stats.len(), 1);
    }

    #[test]
    fn test_clean_flavor_removes_breaks() {
        assert_eq!(
            clean_flavor("でんきを\nためて\u{c}こうげき"),
            "でんきをためてこうげき"
        );
    }

    #[test]
    fn test_stat_ja_known_and_unknown() {
        assert_eq!(stat_ja("speed"), Some("すばやさ"));
        assert_eq!(stat_ja("accuracy"), None);
    }

    #[test]
    fn test_get_pokemon_id() {
        let mut id_map = HashMap::new();
        id_map.insert("Pikachu".to_string(), 25);
        let service = PokemonInfoService::for_test("test://mock".to_string(), id_map);
        assert_eq!(service.get_pokemon_id("Pikachu"), Some(25));
        assert_eq!(service.get_pokemon_id("Unknown"), None);
    }

    #[test]
    fn test_fetch_unknown_pokemon_is_none() {
        let service = PokemonInfoService::for_test("test://mock".to_string(), HashMap::new());
        assert!(service.fetch("Unknown").is_none());
    }

    #[test]
    fn test_fetch_maps_types_and_stats() {
        use httpmock::prelude::*;

        let server = MockServer::start();
        let mock = server.mock(|when, then| {
            when.method(GET).path("/pokemon/25");
            then.status(200)
                .header("content-type", "application/json")
                .body(
                    r#"{
                        "types":[{"slot":1,"type":{"name":"electric","url":"x"}}],
                        "stats":[
                            {"base_stat":35,"stat":{"name":"hp"}},
                            {"base_stat":55,"stat":{"name":"attack"}},
                            {"base_stat":90,"stat":{"name":"speed"}}
                        ]
                    }"#,
                );
        });

        let mut id_map = HashMap::new();
        id_map.insert("Pikachu".to_string(), 25);
        let service = PokemonInfoService::for_test(server.url(""), id_map);

        let info = service.fetch("Pikachu").expect("should fetch");
        assert_eq!(info.types.len(), 1);
        assert_eq!(info.types[0].ja, "でんき");
        assert!(!info.types[0].color.is_empty());
        assert_eq!(info.stats.len(), 3);
        assert_eq!(info.stats[0].label, "HP      ");
        assert_eq!(info.stats[0].value, 35);
        assert_eq!(info.stats[2].label, "すばやさ");
        assert_eq!(info.stats[2].value, 90);
        mock.assert();
    }

    #[test]
    fn test_fetch_description_prefers_ja_then_ja_hrkt() {
        use httpmock::prelude::*;

        let server = MockServer::start();
        // species URL は id 抽出にのみ使う。取得先は base_url + /pokemon-species/25
        let species_url = server.url("/pokemon-species/25");
        server.mock(|when, then| {
            when.method(GET).path("/pokemon/25");
            then.status(200)
                .header("content-type", "application/json")
                .body(format!(
                    r#"{{"types":[],"stats":[],"species":{{"url":"{}"}}}}"#,
                    species_url
                ));
        });
        server.mock(|when, then| {
            when.method(GET).path("/pokemon-species/25");
            then.status(200)
                .header("content-type", "application/json")
                .body(
                    r#"{"flavor_text_entries":[
                        {"flavor_text":"かな\nテキスト","language":{"name":"ja-Hrkt"}},
                        {"flavor_text":"漢字\nテキスト","language":{"name":"ja"}}
                    ]}"#,
                );
        });

        let mut id_map = HashMap::new();
        id_map.insert("Pikachu".to_string(), 25);
        let service = PokemonInfoService::for_test(server.url(""), id_map);

        // ja を優先し、clean_flavor で改行が詰められている
        let info = service.fetch("Pikachu").expect("should fetch");
        assert_eq!(info.description.as_deref(), Some("漢字テキスト"));
    }
}
