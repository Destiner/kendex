//! Letters from other scripts that are drawn as Latin ones, folded to the
//! Latin letters they imitate.
//!
//! Scope, stated plainly because a security control that overclaims is worse
//! than one that is narrow: this covers the scripts an attacker reaches for
//! when hiding an English instruction — Cyrillic, Greek, Armenian, Cherokee,
//! and the Latin-extended, phonetic and small-capital letters that imitate
//! ASCII. It is not the whole of Unicode's confusables data. Fullwidth
//! forms, mathematical alphanumerics and circled letters are not here
//! because NFKC has already collapsed them before this table is consulted.
//!
//! Over-inclusion costs little and under-inclusion is a bypass. Folding a
//! non-Latin letter to ASCII can only produce a rule match when the text
//! around it already spells an English phrase the rules look for — which is
//! the attack. So where a character plausibly imitates a Latin one, it is
//! in. Every fold is counted and reported by `obfuscated-content`, so
//! nothing this table does is silent.

/// Each pair is `(imitators, the Latin letters they imitate)`, one character
/// against one character, in order. A test asserts the alignment, because
/// the two halves are read by index and a short half folds the wrong letters.
const TABLES: &[(&str, &str)] = &[
    // Cyrillic, both cases.
    ("АВЕКМНОРСТУХІЈЅԀԚԜҒҮӀЗ", "ABEKMHOPCTYXIJSDQWFYI3"),
    ("аеорсухіјѕвгдкмнтѵԁԛԝёз", "aeopcyxijsbrdkmhtvdqwe3"),
    // Greek, both cases. Final sigma is in: it is drawn low and round and is
    // the usual stand-in for an `s`.
    ("ΑΒΕΖΗΙΚΜΝΟΡΤΥΧϹϳͿ", "ABEZHIKMNOPTYXCjJ"),
    ("αβγεικνορστυχωςϲ", "abyeikvopotuxwsc"),
    // Armenian.
    ("ոսցօհրբլյպքԱՕՍՏ", "nugohrpljwfUOUS"),
    // Latin extended and phonetic letters — Latin already, and still drawn
    // as a different Latin letter.
    ("ɡɑɩıȷɭɫƅƨʂʈơɓɗɖ", "gaiijllbsstobdd"),
    // Small capitals.
    ("ᴀʙᴄᴅᴇꜰɢʜɪᴊᴋʟᴍɴᴏᴘʀꜱᴛᴜᴠᴡʏᴢ", "abcdefghijklmnoprstuvwyz"),
    // Cherokee, whose syllabary was drawn from Latin letterforms.
    ("ᎠᎡᎢᎪᎫᎬᎯᎷᏂᏊᏓᏔᏕᏙᏞᏢᏣᏦᏴ", "DRTAJEHMHGSWCVLPCKB"),
];

/// The Latin letter `c` imitates, if it imitates one.
pub fn fold(c: char) -> Option<char> {
    if c.is_ascii() {
        return None;
    }
    TABLES.iter().find_map(|(from, to)| {
        from.chars()
            .position(|candidate| candidate == c)
            .and_then(|index| to.chars().nth(index))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The halves of every pair are read by index, so a table whose halves
    /// are different lengths folds letters to the wrong Latin ones.
    #[test]
    fn every_table_maps_one_character_to_one_character() {
        for (from, to) in TABLES {
            assert_eq!(
                from.chars().count(),
                to.chars().count(),
                "misaligned table: {from} / {to}"
            );
        }
    }

    /// A table that folds only the capitals hands an attacker the lowercase
    /// alphabet.
    #[test]
    fn folding_is_case_symmetric_where_both_cases_exist() {
        for (upper, lower) in [('А', 'а'), ('Е', 'е'), ('Т', 'т'), ('Ο', 'ο'), ('Ρ', 'ρ')]
        {
            assert!(fold(upper).is_some(), "{upper} should fold");
            assert!(fold(lower).is_some(), "{lower} should fold");
        }
    }

    #[test]
    fn ascii_is_never_folded() {
        for c in '\0'..='\u{7f}' {
            assert_eq!(fold(c), None, "{c:?} is already Latin");
        }
    }
}
