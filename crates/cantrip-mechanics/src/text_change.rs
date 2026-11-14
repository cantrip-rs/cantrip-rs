//! Changing an object's text (CR 612): the words a text-changing spell replaces, chosen as it resolves (CR 608.2d), and
//! the effect that changes them (`EffectKind::ChangeText`, layer 3).
//!
//! "Replacing all instances of one color word with another or one basic land type with another" is printed on Mind Bend
//! and Trait Doctoring; a card that prints it sets its `labels` to `LABELS`, the two kinds of word it offers. "One color
//! word with another" alone is Sleight of Mind's (and Alter Reality's), "one basic land type with another"
//! alone Magical Hack's (and Spectral Shift's).

use cantrip_api::*;

/// What a "color word or basic land type" choice offers first, as the card's `labels`.
pub const LABELS: &[&str] = &["Replace a color word", "Replace a basic land type"];

const BASIC_LAND_TYPES: [Subtype; 5] = [Subtype::Plains, Subtype::Island, Subtype::Swamp, Subtype::Mountain, Subtype::Forest];

/// One color word to be replaced with another, or one basic land type with another (CR 612.2), chosen by `player`: the
/// kind of word, the word, then the word it becomes, which is a different one.
pub fn color_word_or_basic_land_type(g: &mut dyn Game, player: PlayerId, source: ObjectId) -> Result<(Word, Word), Suspend> {
    let card = g.chars(source).card;
    let kind = crate::choose::one_of(g, player, source, vec![Label::Card(card, 0), Label::Card(card, 1)])?;
    if kind == 0 { color_word(g, player, source) } else { basic_land_type(g, player, source) }
}

/// "One color word with another" (CR 612.2): the color word, then a different one it becomes, chosen by `player`.
pub fn color_word(g: &mut dyn Game, player: PlayerId, source: ObjectId) -> Result<(Word, Word), Suspend> {
    let from = one_color(g, player, source, ColorSet(0b11111), ColorUse::ReplacedWord)?;
    let to = one_color(g, player, source, ColorSet(0b11111 & !ColorSet::of(from).0), ColorUse::ReplacingWord)?;
    Ok((Word::Color(from), Word::Color(to)))
}

/// "One basic land type with another" (CR 612.2): the land type, then a different one it becomes, chosen by `player`.
pub fn basic_land_type(g: &mut dyn Game, player: PlayerId, source: ObjectId) -> Result<(Word, Word), Suspend> {
    let (from, to) = two_basic_land_types(g, player, source, WordUse::Replaced, WordUse::Replacing)?;
    Ok((Word::Subtype(from), Word::Subtype(to)))
}

/// A basic land type, then a different one, chosen by `player`, each asked as its `purpose` says.
pub fn two_basic_land_types(g: &mut dyn Game, player: PlayerId, source: ObjectId, first: WordUse, second: WordUse) -> Result<(Subtype, Subtype), Suspend> {
    let one = one_subtype(g, player, source, BASIC_LAND_TYPES.to_vec(), first)?;
    let other = one_subtype(g, player, source, BASIC_LAND_TYPES.into_iter().filter(|&s| s != one).collect(), second)?;
    Ok((one, other))
}

/// One color of `choices`.
fn one_color(g: &mut dyn Game, player: PlayerId, source: ObjectId, choices: ColorSet, purpose: ColorUse) -> Result<Color, Suspend> {
    let ask = RequestKind::ChooseColors { source, choices, count: 1, purpose };
    let Answer::Colors(colors) = g.ask(player, ask)? else { unreachable!("the core answers a color choice with colors") };
    Ok(colors[0])
}

/// One subtype of `choices`, the word replaced or the word replacing it as `purpose` says.
pub fn one_subtype(g: &mut dyn Game, player: PlayerId, source: ObjectId, choices: Vec<Subtype>, purpose: WordUse) -> Result<Subtype, Suspend> {
    let ask = RequestKind::ChooseSubtype { source, choices, purpose };
    let Answer::Subtype(chosen) = g.ask(player, ask)? else { unreachable!("the core answers a subtype choice with a subtype") };
    Ok(chosen)
}

/// "Change the text of [it] by replacing all instances of `from` with `to`" for `duration` (CR 612.1, 611.2). `object` is the
/// resolving spell's target as `targets::object` returned it, so it is still the object that was targeted (CR 400.7, 608.2b);
/// the effect applies to a permanent or a spell, and goes on applying to the permanent a permanent spell becomes (CR 400.7a).
pub fn change(g: &mut dyn Game, source: ObjectId, object: ObjectId, (from, to): (Word, Word), duration: Duration) {
    g.add_effect(source, object, EffectKind::ChangeText { from, to }, duration);
}
