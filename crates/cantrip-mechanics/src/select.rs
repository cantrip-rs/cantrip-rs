//! Ready-made selection rules: what a card's choice of cards or library search may take, written as its Oracle text words
//! it (docs/engine-design.md, "Selection rules"; `cantrip_api::select` is the machinery). **A new card looks here first.**
//! A rule is a `Selection`: a `Count`, the noun its caption uses, and any number of parts, each a plain function the core
//! runs over the cards chosen so far, both to check the answer and to tell the player, as they pick, which cards may be
//! added. It is asked with `choose::select` (a choice among cards) or `library::search` and the `library::find_to_*`
//! wrappers (a library search).
//!
//! What is here:
//!
//! - **Whole rules for the common wordings:** [`CARD`] ("search your library for a card"), [`BASIC_LAND_CARD`],
//!   [`LAND_CARD`], [`CREATURE_CARD`], [`ARTIFACT_CARD`], [`NONLAND_CARD`], and the same nouns for any count: [`cards`],
//!   [`basic_land_cards`], [`land_cards`], [`creature_cards`], [`artifact_cards`]
//!   (`select::basic_land_cards(up_to(2))`).
//! - **Counts:** [`exactly`], [`up_to`], [`ANY_NUMBER`], [`as_many_as_can`] - "a card" of a search with no quality it could
//!   fail to find is `as_many_as_can(1)`; "a [quality] card" may be left unfound (CR 701.23b), `up_to(1)`.
//! - **Qualities of each card:** [`BASIC_LAND`], [`LAND`], [`CREATURE`], [`ARTIFACT`], [`NONLAND`], [`mana_value_at_most`],
//!   [`named`].
//! - **Rules of the cards together:** [`DIFFERENT_NAMES`] (CR 201.2b), [`total_mana_value_at_most`] (CR 202.3).
//! - **Putting them together:** `Selection::new(count).with(part).with(part).called("noun")`, or a whole rule
//!   `.taking(count)` and `.with(part)` more: `basic_land_cards(up_to(x)).with(DIFFERENT_NAMES)`.
//!
//! **Adding one.** A part is a `SelectFn` - `fn(&dyn Query, &Choice, u32) -> Verdict` - made a `Part` with
//! `cantrip_api::select::part(&(the_fn as SelectFn), n, "words")`; a quality of each card alone is
//! `Verdict::each(choice, |card| ..)`. It reads the game through the `Query` and nothing else, never a captured value: the
//! one number it takes is its data. A rule only one card prints stays in that card's file; one a second printed card uses
//! exactly as written moves here, its doc comment naming both cards, as for any shared mechanic.

pub use cantrip_api::select::{part, tallied};
use cantrip_api::select::Tally;
use cantrip_api::*;

pub use cantrip_api::select::{Count, Part, Selection};

/// Exactly `n` cards ("discard two cards").
pub const fn exactly(n: u32) -> Count {
    Count::Exactly(n)
}

/// From none to `n` cards ("up to two"; a search for cards with a stated quality, which may be left unfound, CR 701.23b).
pub const fn up_to(n: u32) -> Count {
    Count::UpTo(n)
}

/// "Any number of" cards: none, some or all (CR 701.23, 608.2d). Ugin, Eye of the Storms and Nissa, Who Shakes the World
/// search for it; Ao, the Dawn Sky chooses it.
pub const ANY_NUMBER: Count = Count::AnyNumber;

/// `n` cards, or as many as there are to take: a search with no quality to fail at ("search your library for a card",
/// Demonic Tutor; "two cards", CR 701.23b).
pub const fn as_many_as_can(n: u32) -> Count {
    Count::AsManyAsCan(n)
}

fn basic_land(q: &dyn Query, choice: &Choice, _: u32) -> Verdict {
    Verdict::each(choice, |card| crate::lands::is_basic_land_card(q, card))
}

fn land(q: &dyn Query, choice: &Choice, _: u32) -> Verdict {
    Verdict::each(choice, |card| q.chars(card).is(CardType::Land))
}

fn creature(q: &dyn Query, choice: &Choice, _: u32) -> Verdict {
    Verdict::each(choice, |card| q.chars(card).is(CardType::Creature))
}

fn nonland(q: &dyn Query, choice: &Choice, _: u32) -> Verdict {
    Verdict::each(choice, |card| !q.chars(card).is(CardType::Land))
}

fn artifact(q: &dyn Query, choice: &Choice, _: u32) -> Verdict {
    Verdict::each(choice, |card| q.chars(card).is(CardType::Artifact))
}

fn mana_value(q: &dyn Query, choice: &Choice, most: u32) -> Verdict {
    Verdict::each(choice, |card| q.mana_value(card) <= most)
}

fn has_name(q: &dyn Query, choice: &Choice, card: u32) -> Verdict {
    let wanted = CardId(u16::try_from(card).unwrap_or(u16::MAX));
    Verdict::each(choice, |c| crate::library::object_has_name(q, c, wanted))
}

/// "A basic land card" (CR 205.4a, 305.6): a land card with the basic supertype (`lands::is_basic_land_card`). The basic
/// land fetches (Evolving Wilds, Path to Exile, Ghost Quarter) and Fblthp, Knows the Way search for it.
pub const BASIC_LAND: Part = part(&(basic_land as SelectFn), 0, "");

/// "A land card". Crop Rotation, Expedition Map, Golos, Tireless Pilgrim and Vibrance search for it.
pub const LAND: Part = part(&(land as SelectFn), 0, "");

/// "A creature card". Survival of the Fittest, Worldly Tutor and Formidable Speaker search for it.
pub const CREATURE: Part = part(&(creature as SelectFn), 0, "");

/// "A nonland card". Reckoner Shakedown chooses one from a revealed hand (`NONLAND_CARD`); Tidehollow Sculler, Elite
/// Spellbinder and Deep Cavern Bat print the same quality.
pub const NONLAND: Part = part(&(nonland as SelectFn), 0, "");

/// "An artifact card". Tinker, Goblin Engineer, Inventors' Fair and Transmute Artifact search for it.
pub const ARTIFACT: Part = part(&(artifact as SelectFn), 0, "");

/// "With mana value `most` or less" (CR 202.3), each card's read where it is (an {X} in a library or graveyard is 0, CR
/// 202.3e). Trinket Mage (an artifact card, 1) and Green Sun's Zenith (a green creature card, X) search for it.
pub const fn mana_value_at_most(most: u32) -> Part {
    part(&(mana_value as SelectFn), most, "with mana value {n} or less")
}

/// "A card named [`card`]" (CR 201.2a): a card that has the name of the card `card`, one of a split card's names included.
/// Ancient Vendetta and The Rise of Sozin's chapter II search for cards "with that name".
pub const fn named(card: CardId) -> Part {
    part(&(has_name as SelectFn), card.0 as u32, "named {card}")
}

fn different_names(q: &dyn Query, choice: &Choice, _: u32) -> Verdict {
    let chosen: Vec<_> = choice.selected.iter().map(|&c| crate::library::object_names(q, c)).collect();
    let apart = |a: &[cantrip_api::ObjectName], b: &[cantrip_api::ObjectName]| !a.iter().any(|name| b.contains(name));
    // CR 201.2b: each has at least one name, and no two of them a name in common.
    let complete = chosen.iter().enumerate().all(|(i, names)| !names.is_empty() && chosen[..i].iter().all(|other| apart(names, other)));
    let addable = Bits::of(choice.candidates.len(), |i| {
        let names = crate::library::object_names(q, choice.candidates[i]);
        !names.is_empty() && chosen.iter().all(|other| apart(&names, other))
    });
    Verdict { complete, addable }
}

/// "With different names" (CR 201.2b): each card chosen has a name, and no two of them a name in common - a split card
/// shares a name with a card named for either half (CR 201.2a, `same_name`). Fblthp, Knows the Way and Gifts Ungiven
/// search for cards with different names.
pub const DIFFERENT_NAMES: Part = part(&(different_names as SelectFn), 0, "with different names");

fn total_mana_value(q: &dyn Query, choice: &Choice, most: u32) -> Verdict {
    let total: u64 = choice.selected.iter().map(|&c| u64::from(q.mana_value(c))).sum();
    let left = u64::from(most).saturating_sub(total);
    Verdict { complete: total <= u64::from(most), addable: Bits::of(choice.candidates.len(), |i| u64::from(q.mana_value(choice.candidates[i])) <= left) }
}

/// "With total mana value `most` or less" (CR 202.3): the mana values of the cards chosen, each read where it is (an {X} in
/// a library or graveyard is 0, CR 202.3e), add up to at most `most`. Michelangelo's Technique (up to two, 6), Lively Dirge
/// (up to two, 4) and Ao, the Dawn Sky (any number, 4) print it.
pub const fn total_mana_value_at_most(most: u32) -> Part {
    tallied(&(total_mana_value as SelectFn), most, "with total mana value {n} or less", Tally::ManaValueAtMost)
}

/// "A card", any of them, which a search with no quality takes if there is one (CR 701.23b): Demonic Tutor, Vampiric Tutor,
/// Diabolic Intent, Entomb and Imperial Seal search for it.
pub const CARD: Selection = cards(as_many_as_can(1));

/// "A basic land card", which may be left unfound (CR 701.23b): the basic land fetches (Evolving Wilds, Path to Exile,
/// Summon Fenrir, Ghost Quarter).
pub const BASIC_LAND_CARD: Selection = basic_land_cards(up_to(1));

/// "A land card", which may be left unfound (CR 701.23b): Crop Rotation, Expedition Map, Golos, Tireless Pilgrim and Vibrance.
pub const LAND_CARD: Selection = land_cards(up_to(1));

/// "A creature card", which may be left unfound (CR 701.23b): Survival of the Fittest, Worldly Tutor and Formidable Speaker.
pub const CREATURE_CARD: Selection = creature_cards(up_to(1));

/// "An artifact card", which may be left unfound (CR 701.23b): Tinker, Goblin Engineer and Inventors' Fair.
pub const ARTIFACT_CARD: Selection = artifact_cards(up_to(1));

/// "You may choose a nonland card from it" (a revealed or looked-at hand): Reckoner Shakedown, and Deep Cavern Bat's "you
/// may exile a nonland card from it".
pub const NONLAND_CARD: Selection = Selection::of(up_to(1), &[NONLAND]).called("nonland");

/// `count` cards, any of them: Demonic Tutor's "a card" (`CARD`), Ugin, Eye of the Storms' "any number of" cards.
pub const fn cards(count: Count) -> Selection {
    Selection::new(count)
}

/// `count` basic land cards: Settle the Wreckage's "that many basic land cards", Fblthp, Knows the Way's "up to X" (with
/// `DIFFERENT_NAMES`), the basic land fetches' one (`BASIC_LAND_CARD`).
pub const fn basic_land_cards(count: Count) -> Selection {
    Selection::of(count, &[BASIC_LAND]).called("basic land")
}

/// `count` land cards: Crop Rotation's and Expedition Map's one (`LAND_CARD`), and Hexhaven Invigorator's "up to that many
/// land cards".
pub const fn land_cards(count: Count) -> Selection {
    Selection::of(count, &[LAND]).called("land")
}

/// `count` creature cards: Survival of the Fittest's and Worldly Tutor's one (`CREATURE_CARD`).
pub const fn creature_cards(count: Count) -> Selection {
    Selection::of(count, &[CREATURE]).called("creature")
}

/// `count` artifact cards: Tinker's and Goblin Engineer's one (`ARTIFACT_CARD`), and Trinket Mage's one with
/// `mana_value_at_most(1)`.
pub const fn artifact_cards(count: Count) -> Selection {
    Selection::of(count, &[ARTIFACT]).called("artifact")
}
