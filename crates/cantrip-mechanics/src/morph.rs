//! Morph [cost] (CR 702.37a): "You may cast this card as a 2/2 face-down creature with no text, no name, no subtypes, and
//! no mana cost by paying {3} rather than paying its mana cost", and turn it face up any time you have priority by paying
//! its morph cost (CR 702.37e). Zombie Cutthroat ("Morph—Pay 5 life"), Putrid Raptor ("Morph—Discard a Zombie card") and
//! Raven Guild Initiate ("Morph—Return a Bird you control to its owner's hand") print it.
//!
//! A card with morph lists `way(cost, text)` among its `ways`, `cost` its morph cost (mana, or any other cost: CR 118.1),
//! `text` the printed keyword line. Casting it face down and turning it face up are the rules' (CR 708, 116.2b): the way
//! casts the card as `intrinsic::MORPHED` for {3} (`cantrip_api::face_down_way`), and any face-down permanent that would have
//! morph if it were face up - cast face down, manifested or cloaked (CR 701.40c, 701.58c) - may be turned face up for this
//! cost as a special action (`intrinsic::TURN_UP_MORPH_TEXT`).
//!
//! ```ignore
//! ways: &[morph::way(Cost::mana(mana("{4}{W}")), "Morph {4}{W}")],
//! ways: &[morph::way(Cost { life: 5, ..Cost::NONE }, "Morph\u{2014}Pay 5 life.")],
//! ```

use cantrip_api::*;

/// Morph `cost`: cast face down for {3} from wherever the card could be cast (CR 702.37c), turned face up for `cost`
/// (CR 702.37e). `text` is the keyword as printed ("Morph {4}{W}").
pub const fn way(cost: Cost, text: &'static str) -> WayDef {
    face_down_way(WayKind::Morph, cost, text)
}
