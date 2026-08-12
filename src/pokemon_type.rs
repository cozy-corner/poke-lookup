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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_type_ja_known_and_unknown() {
        assert_eq!(type_ja("fire"), Some("ほのお"));
        assert_eq!(type_ja("flying"), Some("ひこう"));
        assert_eq!(type_ja("stellar"), None);
    }
}
