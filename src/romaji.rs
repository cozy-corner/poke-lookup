//! カタカナ名を、skim のマッチ用ローマ字表記に変換する。
//!
//! skim のマッチはファジー（部分列）マッチなので、長音を省いた入力
//! （buusutaa に対する busuta）は生成しなくてもヒットする。一方
//! husigi は fushigi の部分列にならないため、ヘボン式と訓令式の
//! 両方を生成する。

/// ローマ字の表記方式
#[derive(Clone, Copy, PartialEq, Eq)]
enum Style {
    Hepburn,
    Kunrei,
}

/// (カタカナ, ヘボン式, 訓令式)
const SYLLABLES: &[(&str, &str, &str)] = &[
    ("ア", "a", "a"),
    ("イ", "i", "i"),
    ("ウ", "u", "u"),
    ("エ", "e", "e"),
    ("オ", "o", "o"),
    ("カ", "ka", "ka"),
    ("キ", "ki", "ki"),
    ("ク", "ku", "ku"),
    ("ケ", "ke", "ke"),
    ("コ", "ko", "ko"),
    ("サ", "sa", "sa"),
    ("シ", "shi", "si"),
    ("ス", "su", "su"),
    ("セ", "se", "se"),
    ("ソ", "so", "so"),
    ("タ", "ta", "ta"),
    ("チ", "chi", "ti"),
    ("ツ", "tsu", "tu"),
    ("テ", "te", "te"),
    ("ト", "to", "to"),
    ("ナ", "na", "na"),
    ("ニ", "ni", "ni"),
    ("ヌ", "nu", "nu"),
    ("ネ", "ne", "ne"),
    ("ノ", "no", "no"),
    ("ハ", "ha", "ha"),
    ("ヒ", "hi", "hi"),
    ("フ", "fu", "hu"),
    ("ヘ", "he", "he"),
    ("ホ", "ho", "ho"),
    ("マ", "ma", "ma"),
    ("ミ", "mi", "mi"),
    ("ム", "mu", "mu"),
    ("メ", "me", "me"),
    ("モ", "mo", "mo"),
    ("ヤ", "ya", "ya"),
    ("ユ", "yu", "yu"),
    ("ヨ", "yo", "yo"),
    ("ラ", "ra", "ra"),
    ("リ", "ri", "ri"),
    ("ル", "ru", "ru"),
    ("レ", "re", "re"),
    ("ロ", "ro", "ro"),
    ("ワ", "wa", "wa"),
    ("ヲ", "o", "o"),
    ("ン", "n", "n"),
    ("ガ", "ga", "ga"),
    ("ギ", "gi", "gi"),
    ("グ", "gu", "gu"),
    ("ゲ", "ge", "ge"),
    ("ゴ", "go", "go"),
    ("ザ", "za", "za"),
    ("ジ", "ji", "zi"),
    ("ズ", "zu", "zu"),
    ("ゼ", "ze", "ze"),
    ("ゾ", "zo", "zo"),
    ("ダ", "da", "da"),
    ("ヂ", "ji", "zi"),
    ("ヅ", "zu", "zu"),
    ("デ", "de", "de"),
    ("ド", "do", "do"),
    ("バ", "ba", "ba"),
    ("ビ", "bi", "bi"),
    ("ブ", "bu", "bu"),
    ("ベ", "be", "be"),
    ("ボ", "bo", "bo"),
    ("パ", "pa", "pa"),
    ("ピ", "pi", "pi"),
    ("プ", "pu", "pu"),
    ("ペ", "pe", "pe"),
    ("ポ", "po", "po"),
    ("ヴ", "vu", "vu"),
    // 拗音の一部として現れなかった小書きカナ
    ("ァ", "a", "a"),
    ("ィ", "i", "i"),
    ("ゥ", "u", "u"),
    ("ェ", "e", "e"),
    ("ォ", "o", "o"),
    ("ャ", "ya", "ya"),
    ("ュ", "yu", "yu"),
    ("ョ", "yo", "yo"),
];

fn lookup(kana: &str, style: Style) -> Option<&'static str> {
    SYLLABLES
        .iter()
        .find(|(k, _, _)| *k == kana)
        .map(|(_, hepburn, kunrei)| match style {
            Style::Hepburn => *hepburn,
            Style::Kunrei => *kunrei,
        })
}

fn to_romaji(katakana: &str, style: Style) -> String {
    let mut out = String::new();
    for c in katakana.chars() {
        match lookup(&c.to_string(), style) {
            Some(r) => out.push_str(r),
            // 変換表にない文字は落とさずそのまま通す
            None => out.push(c),
        }
    }
    out
}

/// カタカナ名から、マッチ用のローマ字表記を返す（ヘボン式・訓令式、重複除去済み）
#[allow(dead_code)] // Task 4 で interactive.rs から使用する
pub fn variants(katakana: &str) -> Vec<String> {
    let hepburn = to_romaji(katakana, Style::Hepburn);
    let kunrei = to_romaji(katakana, Style::Kunrei);

    if hepburn == kunrei {
        vec![hepburn]
    } else {
        vec![hepburn, kunrei]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_single_kana() {
        assert_eq!(variants("ヒトカゲ"), vec!["hitokage"]);
        assert_eq!(variants("ピカ"), vec!["pika"]);
        assert_eq!(variants("ン"), vec!["n"]);
    }

    #[test]
    fn test_hepburn_and_kunrei_differ() {
        assert_eq!(variants("フシギ"), vec!["fushigi", "husigi"]);
        assert_eq!(variants("ツ"), vec!["tsu", "tu"]);
        assert_eq!(variants("チ"), vec!["chi", "ti"]);
        assert_eq!(variants("ジ"), vec!["ji", "zi"]);
    }

    #[test]
    fn test_identical_styles_are_deduped() {
        // 両式で差が出ない名前は 1 件だけ返る
        assert_eq!(variants("カメ").len(), 1);
    }

    #[test]
    fn test_unknown_char_passes_through() {
        // 変換表にない文字は落とさずそのまま通す
        assert_eq!(variants("ポリゴン2"), vec!["porigon2"]);
    }
}
