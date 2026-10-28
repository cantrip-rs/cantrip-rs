//! Rules text as it reads now (CR 109.3, 612): the printed text with the words a text-changing effect replaced. The
//! engine reads a card's words through `Query::words`; this is the same change made to the text a player reads. The card's
//! name is never changed (CR 612.2), and nor is reminder text, which is no rules text.

use cantrip_engine::*;

/// `text` as `words` have it: every color word, basic land type and creature type they change, singular or plural, in
/// the case it was printed in, outside the card's `name` and parenthesized reminder text.
pub fn changed_text(text: &str, name: &str, words: Words) -> String {
    if words.is_printed() {
        return text.to_string();
    }
    let changed: Vec<Subtype> = Subtype::ALL.into_iter().filter(|&s| words.subtype(s) != s).collect();
    let short = name.split(',').next().unwrap_or(name);
    let mut out = String::with_capacity(text.len());
    let mut depth = 0usize;
    let mut i = 0;
    while i < text.len() {
        let rest = &text[i..];
        if depth == 0 && (rest.starts_with(name) || (!short.is_empty() && rest.starts_with(short))) {
            let skip = if rest.starts_with(name) { name.len() } else { short.len() };
            out.push_str(&rest[..skip]);
            i += skip;
            continue;
        }
        let c = rest.chars().next().unwrap();
        if !c.is_alphabetic() {
            match c {
                '(' => depth += 1,
                ')' => depth = depth.saturating_sub(1),
                _ => {}
            }
            out.push(c);
            i += c.len_utf8();
            continue;
        }
        let len = rest.find(|ch: char| !(ch.is_alphanumeric() || ch == '\'' || ch == '-')).unwrap_or(rest.len());
        let word = &rest[..len];
        // A word ends before an apostrophe-s ("Goblin's"), which goes on after it unchanged.
        let (word, len) = match word.find('\'') {
            Some(at) if !word.contains("'t") => (&word[..at], at),
            _ => (word, len),
        };
        match (depth == 0).then(|| replaced(word, words, &changed)).flatten() {
            Some(new) => out.push_str(&new),
            None => out.push_str(word),
        }
        i += len.max(1);
    }
    out
}

/// The word `word` becomes, if `words` change it: a color word or a subtype, alone, in "non[color]" (Mind Bend's
/// "nonblack creature" to "nongreen creature") or in a landwalk ability ("forestwalk" to "islandwalk", CR 702.14a).
fn replaced(word: &str, words: Words, changed: &[Subtype]) -> Option<String> {
    for prefix in ["non", "Non"] {
        if let Some(rest) = word.strip_prefix(prefix)
            && Color::ALL.into_iter().any(|c| c.name().eq_ignore_ascii_case(rest))
        {
            return replaced(rest, words, changed).map(|new| format!("{prefix}{new}"));
        }
    }
    if let Some(land) = word.strip_suffix("walk")
        && let Some(&s) = changed.iter().find(|s| s.name().eq_ignore_ascii_case(land))
    {
        return Some(format!("{}walk", in_case_of(land, words.subtype(s).name())));
    }
    if let Some(color) = Color::ALL.into_iter().find(|c| c.name().eq_ignore_ascii_case(word)) {
        let now = words.color(color);
        return (now != color).then(|| in_case_of(word, now.name()));
    }
    for &s in changed {
        let now = words.subtype(s);
        if word == s.name() {
            return Some(now.name().to_string());
        }
        if word == plural(s.name()) {
            return Some(plural(now.name()));
        }
    }
    None
}

/// `new` written as `old` was: lower case for a color word mid-sentence, capitalized at its start.
fn in_case_of(old: &str, new: &str) -> String {
    if old.chars().next().is_some_and(char::is_lowercase) { new.to_lowercase() } else { new.to_string() }
}

/// How a subtype is written in the plural: "Goblins", "Elves", "Plains".
pub(crate) fn plural(name: &str) -> String {
    match name {
        "Elf" => "Elves".into(),
        "Dwarf" => "Dwarves".into(),
        "Wolf" => "Wolves".into(),
        "Werewolf" => "Werewolves".into(),
        "Fungus" => "Fungi".into(),
        "Ox" => "Oxen".into(),
        "Mouse" => "Mice".into(),
        "Cyclops" => "Cyclopes".into(),
        "Plains" | "Sheep" | "Fish" | "Merfolk" | "Kithkin" | "Moonfolk" | "Samurai" | "Kor" | "Aetherborn" => name.into(),
        _ if name.ends_with('s') || name.ends_with('x') || name.ends_with("ch") || name.ends_with("sh") => format!("{name}es"),
        _ if name.ends_with('y') && !name.ends_with("ey") && !name.ends_with("ay") && !name.ends_with("oy") => {
            format!("{}ies", &name[..name.len() - 1])
        }
        _ => format!("{name}s"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn change(from: Word, to: Word) -> Words {
        Words::NONE.changed(from, to)
    }

    #[test]
    fn a_land_type_in_a_landwalk_and_a_color_after_non_are_replaced() {
        let w = change(Word::Subtype(Subtype::Island), Word::Subtype(Subtype::Swamp));
        assert_eq!(changed_text("Other Merfolk have islandwalk.\nIslandwalk", "Lord", w), "Other Merfolk have swampwalk.\nSwampwalk");
        let w = change(Word::Color(Color::Black), Word::Color(Color::Green));
        assert_eq!(changed_text("Destroy target nonblack creature.", "Doom Blade", w), "Destroy target nongreen creature.");
    }

    #[test]
    fn a_color_word_is_replaced_in_its_case_and_a_mana_symbol_is_not() {
        let w = change(Word::Color(Color::Blue), Word::Color(Color::Red));
        assert_eq!(
            changed_text("Counter target spell if it's blue. Blue creatures get {U}.", "Pyroblast", w),
            "Counter target spell if it's red. Red creatures get {U}."
        );
    }

    #[test]
    fn a_creature_type_is_replaced_singular_and_plural_and_the_name_and_reminder_text_are_not() {
        let w = change(Word::Subtype(Subtype::Elf), Word::Subtype(Subtype::Goblin));
        assert_eq!(
            changed_text("Other Elves you control get +1/+1. Elf Lord's Elf (an Elf).", "Elf Lord", w),
            "Other Goblins you control get +1/+1. Elf Lord's Goblin (an Elf)."
        );
    }

    #[test]
    fn a_land_type_is_replaced_in_a_fetch_lands_text() {
        let w = change(Word::Subtype(Subtype::Mountain), Word::Subtype(Subtype::Island));
        assert_eq!(
            changed_text("Search your library for a Mountain or Plains card. Sacrifice Arid Mesa.", "Arid Mesa", w),
            "Search your library for a Island or Plains card. Sacrifice Arid Mesa."
        );
    }
}
