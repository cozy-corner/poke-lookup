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

/// タイプの英語スラッグ → 漢字表記。IME が「でんき」を「電気」に変換してしまうため、
/// 変換後の入力でも引けるよう検索用のエイリアスとして持つ。
/// カタカナ表記のタイプ（ノーマル・エスパーなど）は漢字表記を持たないので None。
pub fn type_kanji(slug: &str) -> Option<&'static str> {
    Some(match slug {
        "fire" => "炎",
        "water" => "水",
        "electric" => "電気",
        "grass" => "草",
        "ice" => "氷",
        "fighting" => "格闘",
        "poison" => "毒",
        "ground" => "地面",
        "flying" => "飛行",
        "bug" => "虫",
        "rock" => "岩",
        "dark" => "悪",
        "steel" => "鋼",
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

    #[test]
    fn test_type_kanji_known_and_unknown() {
        assert_eq!(type_kanji("electric"), Some("電気"));
        assert_eq!(type_kanji("grass"), Some("草"));
        // カタカナ表記のタイプと未知 slug は漢字表記なし
        assert_eq!(type_kanji("normal"), None);
        assert_eq!(type_kanji("stellar"), None);
    }
}
