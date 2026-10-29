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
//! - **Qualities of each card:** [`BASIC_LAND`], [`LAND`], [`CREATURE`], [`ARTIFACT`], [`NONLAND`], [`mana_value_at_most`], [`mana_value_exactly`],
//!   [`named`].
//! - **Rules of the cards together:** [`DIFFERENT_NAMES`] (CR 201.2b), [`DIFFERENT_MANA_VALUES`] (CR 202.3),
//!   [`total_mana_value_at_most`] (CR 202.3), [`SHARE_A_CARD_TYPE`] (CR 205.2a).
//! - **Targets together:** a rule of the cards together is a set rule (`set_part`, with a `LargestFn` that finds a largest
//!   set it allows), and judges a target slot's targets together too (`TargetSpec::together`, CR 601.2c): "any number of
//!   target creature cards with total mana value X or less" is `.together(select::total_mana_value_at_most(TargetSpec::COUNTED))`,
//!   `TargetSpec::COUNTED` read as the slot's count (the X announced). A quality of each target ("with mana value X or
//!   less") is the slot's filter's, reading `Query::announced_x`, never `together`.
//! - **Every part is downward-closed** (`Part`): any of the cards of a set it allows are allowed too.
//! - **Putting them together:** `Selection::new(count).with(part).with(part).called("noun")`, or a whole rule
//!   `.taking(count)` and `.with(part)` more: `basic_land_cards(up_to(x)).with(DIFFERENT_NAMES)`.
//!
//! **Adding one.** A part is a `SelectFn` - `fn(&dyn Query, &Choice, u32) -> Verdict` - made a `Part` with
//! `cantrip_api::select::part(&(the_fn as SelectFn), n, "words")`; a quality of each card alone is
//! `Verdict::each(choice, |card| ..)`. It reads the game through the `Query` and nothing else, never a captured value: the
//! one number it takes is its data. A rule only one card prints stays in that card's file; one a second printed card uses
//! exactly as written moves here, its doc comment naming both cards, as for any shared mechanic.

pub use cantrip_api::select::{LargestFn, part, set_part, set_tallied, tallied};
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

fn another_creature_or_artifact(q: &dyn Query, choice: &Choice, _: u32) -> Verdict {
    Verdict::each(choice, |id| id != choice.source && (q.chars(id).is(CardType::Creature) || q.chars(id).is(CardType::Artifact)))
}

fn another_creature(q: &dyn Query, choice: &Choice, _: u32) -> Verdict {
    Verdict::each(choice, |id| id != choice.source && q.chars(id).is(CardType::Creature))
}

fn forest(q: &dyn Query, choice: &Choice, word: u32) -> Verdict {
    let wanted = crate::lands::word(q, choice.source, u16::try_from(word).unwrap_or(u16::MAX));
    Verdict::each(choice, |card| q.chars(card).has_subtype(wanted))
}

fn mana_value(q: &dyn Query, choice: &Choice, most: u32) -> Verdict {
    Verdict::each(choice, |card| q.mana_value(card) <= most)
}

fn mana_value_is(q: &dyn Query, choice: &Choice, exactly: u32) -> Verdict {
    Verdict::each(choice, |card| q.mana_value(card) == exactly)
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

/// "A Forest card" (CR 205.3i, 305.6): a land card with the subtype Forest, basic or not (a Stomping Ground is one), the type read
/// through the words of the text of the card that asks (CR 612.2) and a card as it is where it is. Nature's Lore, Three Visits and
/// Wood Elves search for it.
pub const FOREST: Part = part(&(forest as SelectFn), crate::lands::sub(Subtype::Forest) as u32, "");

/// "A creature card". Survival of the Fittest, Worldly Tutor and Formidable Speaker search for it.
pub const CREATURE: Part = part(&(creature as SelectFn), 0, "");

/// "A nonland card". Reckoner Shakedown chooses one from a revealed hand (`NONLAND_CARD`); Tidehollow Sculler, Elite
/// Spellbinder and Deep Cavern Bat print the same quality.
pub const NONLAND: Part = part(&(nonland as SelectFn), 0, "");

/// "An artifact card". Tinker, Goblin Engineer, Inventors' Fair and Transmute Artifact search for it.
pub const ARTIFACT: Part = part(&(artifact as SelectFn), 0, "");

/// "Another creature or artifact": a creature or artifact permanent other than the object whose text asks (`Choice::source`).
/// The Sackville-Bagginses and Automated Warfare System sacrifice one.
pub const ANOTHER_CREATURE_OR_ARTIFACT: Part = part(&(another_creature_or_artifact as SelectFn), 0, "");

/// "Another creature": a creature permanent other than the object whose text asks (`Choice::source`). Bolg of the North and
/// Sephiroth, Fabled SOLDIER sacrifice one.
pub const ANOTHER_CREATURE: Part = part(&(another_creature as SelectFn), 0, "");

/// "With mana value `most` or less" (CR 202.3), each card's read where it is (an {X} in a library or graveyard is 0, CR
/// 202.3e). Trinket Mage (an artifact card, 1) and Green Sun's Zenith (a green creature card, X) search for it. Of targets,
/// it is the slot's filter's to ask (`TargetSpec::together`): Agadeem's Awakening's "mana value X or less" reads
/// `Query::announced_x` there.
pub const fn mana_value_at_most(most: u32) -> Part {
    part(&(mana_value as SelectFn), most, "with mana value {n} or less")
}

/// "With the same mana value as" a card or permanent, `exactly` (CR 202.3), each card's read where it is (an {X} in a library is
/// 0, CR 202.3e). Drift of Phantasms's transmute and Fleshwrither's transfigure search for it.
pub const fn mana_value_exactly(exactly: u32) -> Part {
    part(&(mana_value_is as SelectFn), exactly, "with mana value {n}")
}

/// "A card named [`card`]" (CR 201.2a): a card that has the name of the card `card`, one of a split card's names included.
/// Ancient Vendetta and The Rise of Sozin's chapter II search for cards "with that name".
pub const fn named(card: CardId) -> Part {
    part(&(has_name as SelectFn), card.0 as u32, "named {card}")
}

fn different_names(q: &dyn Query, choice: &Choice, _: u32) -> Verdict {
    let chosen: Vec<Vec<&'static str>> = choice.selected.iter().map(|&c| crate::library::object_names(q, c)).collect();
    let apart = |a: &[&'static str], b: &[&'static str]| !a.iter().any(|name| b.contains(name));
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
/// search for cards with different names; Behold the Sinister Six! targets "up to six target creature cards with different
/// names".
pub const DIFFERENT_NAMES: Part = set_part(&(different_names as SelectFn), &(largest_different_names as LargestFn), 0, "with different names");

/// A largest set of cards with different names: the most cards no two of which share a name. A card's names are its own
/// (CR 201.2a-b: a split card's halves', a Room's doors'), and no name is two printed cards', so the cards that share names fall
/// into groups each within the names of one printed card (two for a split card or a Room, five at most): a largest set is
/// the largest of each group together, each found by trying the sets of names the group's cards have.
fn largest_different_names(q: &dyn Query, choice: &Choice, _: u32) -> Vec<ObjectId> {
    let name_sets: Vec<Vec<&'static str>> = choice.candidates.iter().map(|&c| crate::library::object_names(q, c)).collect();
    largest_name_set(&name_sets).into_iter().map(|i| choice.candidates[i]).collect()
}

/// Indices of a largest set of objects whose name sets have no names in common (CR 201.2a-b). Used by
/// `DIFFERENT_NAMES` and `lands::differently_named_lands` (Emil, Vastlands Roamer).
pub fn largest_name_set(name_sets: &[Vec<&'static str>]) -> Vec<usize> {
    let cards: Vec<(usize, &[&'static str])> =
        name_sets.iter().enumerate().filter(|(_, names)| !names.is_empty()).map(|(i, names)| (i, names.as_slice())).collect();
    // The names, and the group each is in, joined through the cards that have several.
    let mut names: Vec<&'static str> = Vec::new();
    for &(_, own) in &cards {
        for &name in own {
            if !names.contains(&name) {
                names.push(name);
            }
        }
    }
    fn root(group: &mut [usize], mut i: usize) -> usize {
        while group[i] != i {
            group[i] = group[group[i]];
            i = group[i];
        }
        i
    }
    let index = |name: &str| names.iter().position(|&n| n == name).unwrap_or(0);
    let mut group: Vec<usize> = (0..names.len()).collect();
    for &(_, own) in &cards {
        for pair in own.windows(2) {
            let (a, b) = (root(&mut group, index(pair[0])), root(&mut group, index(pair[1])));
            group[a] = b;
        }
    }
    let mut kept: Vec<usize> = Vec::new();
    for r in 0..names.len() {
        if root(&mut group, r) != r {
            continue;
        }
        // The group's names, and each different set of them a card has (with the first such card), as sets of their places.
        let members: Vec<usize> = (0..names.len()).filter(|&i| root(&mut group, i) == r).collect();
        let mut sets: Vec<(Vec<usize>, usize)> = Vec::new();
        for &(i, own) in &cards {
            if root(&mut group, index(own[0])) != r {
                continue;
            }
            let mut set: Vec<usize> = own.iter().filter_map(|&name| members.iter().position(|&x| x == index(name))).collect();
            set.sort();
            set.dedup();
            if !sets.iter().any(|(s, _)| *s == set) {
                sets.push((set, i));
            }
        }
        kept.extend(most_apart(&sets, members.len()));
    }
    kept.sort();
    kept
}

/// The cards of the most of `sets` (sets of a group's `names` places, each with its card) no two of which share a name: each
/// tried in turn with what it leaves, remembered by the names used - as many tries as there are sets of names among the
/// group's cards, which the one printed card the group's names are bounds (CR 201.2a-b).
fn most_apart(sets: &[(Vec<usize>, usize)], names: usize) -> Vec<usize> {
    type Memo = std::collections::HashMap<(usize, Vec<bool>), usize>;
    fn go(sets: &[(Vec<usize>, usize)], at: usize, used: &mut Vec<bool>, memo: &mut Memo) -> usize {
        if at == sets.len() {
            return 0;
        }
        if let Some(&n) = memo.get(&(at, used.clone())) {
            return n;
        }
        let skip = go(sets, at + 1, used, memo);
        let take = if sets[at].0.iter().all(|&n| !used[n]) {
            sets[at].0.iter().for_each(|&n| used[n] = true);
            let n = 1 + go(sets, at + 1, used, memo);
            sets[at].0.iter().for_each(|&n| used[n] = false);
            n
        } else {
            0
        };
        let best = skip.max(take);
        memo.insert((at, used.clone()), best);
        best
    }
    let mut memo = Memo::new();
    let mut used = vec![false; names];
    let mut chosen = Vec::new();
    for at in 0..sets.len() {
        if !sets[at].0.iter().all(|&n| !used[n]) {
            continue;
        }
        let best = go(sets, at, &mut used, &mut memo);
        sets[at].0.iter().for_each(|&n| used[n] = true);
        let taking = 1 + go(sets, at + 1, &mut used, &mut memo);
        if taking == best {
            chosen.push(sets[at].1);
        } else {
            sets[at].0.iter().for_each(|&n| used[n] = false);
        }
    }
    chosen
}

fn different_mana_values(q: &dyn Query, choice: &Choice, _: u32) -> Verdict {
    let chosen: Vec<u32> = choice.selected.iter().map(|&c| q.mana_value(c)).collect();
    let complete = chosen.iter().enumerate().all(|(i, v)| !chosen[..i].contains(v));
    let addable = Bits::of(choice.candidates.len(), |i| !chosen.contains(&q.mana_value(choice.candidates[i])));
    Verdict { complete, addable }
}

/// "With different mana values" (CR 202.3): no two of the cards chosen have the same mana value, each read where it is (an
/// {X} in a library or graveyard is 0, CR 202.3e). Long Rest's "X target cards with different mana values" and Agadeem's
/// Awakening's "any number of target creature cards that each have a different mana value X or less" (its "X or less" the
/// slot's filter's) print it of targets.
pub const DIFFERENT_MANA_VALUES: Part =
    set_part(&(different_mana_values as SelectFn), &(largest_different_mana_values as LargestFn), 0, "with different mana values");

fn share_a_card_type(q: &dyn Query, choice: &Choice, _: u32) -> Verdict {
    let all_have = |t: CardType| choice.selected.iter().all(|&c| q.chars(c).is(t));
    let complete = choice.selected.is_empty() || CardType::ALL.into_iter().any(all_have);
    let addable = Bits::of(choice.candidates.len(), |i| {
        let candidate = q.chars(choice.candidates[i]);
        choice.selected.is_empty() || CardType::ALL.into_iter().any(|t| all_have(t) && candidate.is(t))
    });
    Verdict { complete, addable }
}

/// "That share a card type" (CR 205.2a, 205.2): some one card type is a type of every object chosen, each read as it is where
/// it is. A single object shares a type with itself, and any of the objects of a set that does share one share it too.
/// Burglar's Plot's "two target nonland permanents that share a card type" and Shifting Loyalties' "two target permanents
/// that share a card type" print it of targets.
pub const SHARE_A_CARD_TYPE: Part = set_part(&(share_a_card_type as SelectFn), &(largest_sharing_a_card_type as LargestFn), 0, "that share a card type");

/// A largest set that shares a card type: the candidates of the card type the most of them have.
fn largest_sharing_a_card_type(q: &dyn Query, choice: &Choice, _: u32) -> Vec<ObjectId> {
    let with = |t: CardType| choice.candidates.iter().copied().filter(|&c| q.chars(c).is(t)).collect::<Vec<_>>();
    CardType::ALL.into_iter().map(with).max_by_key(Vec::len).unwrap_or_default()
}

/// A largest set with different mana values: the first card of each mana value.
fn largest_different_mana_values(q: &dyn Query, choice: &Choice, _: u32) -> Vec<ObjectId> {
    cantrip_api::select::largest_distinct(choice, |c| q.mana_value(c))
}

/// A largest set within a total mana value: the cards of the smallest mana values while they fit.
fn largest_within_total(q: &dyn Query, choice: &Choice, most: u32) -> Vec<ObjectId> {
    cantrip_api::select::largest_within(choice, |c| u64::from(q.mana_value(c)), u64::from(most))
}

fn total_mana_value(q: &dyn Query, choice: &Choice, most: u32) -> Verdict {
    let total: u64 = choice.selected.iter().map(|&c| u64::from(q.mana_value(c))).sum();
    let left = u64::from(most).saturating_sub(total);
    Verdict { complete: total <= u64::from(most), addable: Bits::of(choice.candidates.len(), |i| u64::from(q.mana_value(choice.candidates[i])) <= left) }
}

/// "With total mana value `most` or less" (CR 202.3): the mana values of the cards chosen, each read where it is (an {X} in
/// a library or graveyard is 0, CR 202.3e), add up to at most `most`. Michelangelo's Technique (up to two, 6), Lively Dirge
/// (up to two, 4) and Ao, the Dawn Sky (any number, 4) print it of cards chosen; The Rise of Sozin's Fire Lord Sozin
/// (any number, X), Patch Up (up to three, 3) and Kairi, the Swirling Sky (any number, 6) of targets.
pub const fn total_mana_value_at_most(most: u32) -> Part {
    set_tallied(&(total_mana_value as SelectFn), &(largest_within_total as LargestFn), most, "with total mana value {n} or less", Tally::ManaValueAtMost)
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

/// `count` Forest cards: Wood Elves's one (`FOREST_CARD`), which may be left unfound (CR 701.23b).
pub const fn forest_cards(count: Count) -> Selection {
    Selection::of(count, &[FOREST]).called("Forest")
}

/// "A Forest card", which may be left unfound (CR 701.23b): Nature's Lore, Three Visits and Wood Elves search for it.
pub const FOREST_CARD: Selection = forest_cards(up_to(1));

/// `count` land cards: Crop Rotation's and Expedition Map's one (`LAND_CARD`), and Hexhaven Invigorator's "up to that many
/// land cards".
pub const fn land_cards(count: Count) -> Selection {
    Selection::of(count, &[LAND]).called("land")
}

/// `count` creature cards: Survival of the Fittest's and Worldly Tutor's one (`CREATURE_CARD`).
pub const fn creature_cards(count: Count) -> Selection {
    Selection::of(count, &[CREATURE]).called("creature")
}

/// `count` of "another creature or artifact" (CR 701.21a), chosen as an effect resolves: The Sackville-Bagginses's and
/// Automated Warfare System's "you may sacrifice" is `up_to(1)`.
pub const fn another_creature_or_artifact_permanents(count: Count) -> Selection {
    Selection::of(count, &[ANOTHER_CREATURE_OR_ARTIFACT]).called("other creature or artifact")
}

/// `count` of "another creature" (CR 701.21a), chosen as an effect resolves: Bolg of the North's and Sephiroth, Fabled SOLDIER's
/// "you may sacrifice" is `up_to(1)`.
pub const fn another_creature_permanents(count: Count) -> Selection {
    Selection::of(count, &[ANOTHER_CREATURE]).called("other creature")
}

/// `count` artifact cards: Tinker's and Goblin Engineer's one (`ARTIFACT_CARD`), and Trinket Mage's one with
/// `mana_value_at_most(1)`.
pub const fn artifact_cards(count: Count) -> Selection {
    Selection::of(count, &[ARTIFACT]).called("artifact")
}

#[cfg(test)]
mod name_set_tests {
    use super::largest_name_set;

    #[test]
    fn largest_name_set_handles_overlapping_and_nameless_objects() {
        let names = [
            vec!["A", "B"],
            vec!["A"],
            vec!["B"],
            vec!["C"],
            vec![],
        ];
        assert_eq!(largest_name_set(&names), vec![1, 2, 3]);
    }
}
