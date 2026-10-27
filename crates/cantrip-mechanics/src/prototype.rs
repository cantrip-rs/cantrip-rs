//! Prototype (CR 702.160a, 718): "A player who casts a spell with prototype can choose to cast that card 'prototyped.' If
//! they do, the alternative set of its power, toughness, and mana cost characteristics are used." Blitz Automaton, Goring
//! Warplow, Phyrexian Fleshgorger and Rust Goliath print it.
//!
//! Prototype is the core's: the inset frame's values are a characteristic of the card, generated from its content's
//! `"prototype": {"manaCost": "{2}{R}", "power": 3, "toughness": 2}` (`CardDef::prototype`), and the core makes the
//! prototyped face from the card (`cantrip_api::prototyped_face`: the card with only that mana cost, power and toughness, and
//! the color the mana cost gives, CR 718.3b, 718.5), offers casting it as that face wherever the card could be cast
//! (`WAY_PROTOTYPE`, judged as the face, CR 718.3a, for its mana cost or an alternative cost: the ruling), and has the spell
//! and the permanent it becomes keep it as their copiable values (CR 718.2a, 718.3c-d) until they leave the stack or the
//! battlefield (CR 718.4). So a card with nothing but prototype and the core's keywords is its generated constant alone:
//!
//! ```ignore
//! pub static CARD: CardDef = CardDef { ..oracle::BLITZ_AUTOMATON };
//! ```
//!
//! and a card that prints more writes it as any card does: the prototyped face is made from the card, abilities and all.

use cantrip_api::*;

/// Whether `object` - a spell or a permanent - is prototyped: a spell cast prototyped, the permanent it became, or a copy of
/// either (CR 718.3c-d: "any rule or effect that refers to a prototyped spell [or permanent] refers to the copy as well").
pub fn prototyped(q: &dyn Query, object: ObjectId) -> bool {
    q.card(q.chars(object).card).prototyped
}
