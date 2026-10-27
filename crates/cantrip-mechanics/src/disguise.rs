//! Disguise [cost] (CR 702.168a): "You may cast this card as a 2/2 face-down creature with ward {2}, no name, no subtypes,
//! and no mana cost by paying {3} rather than paying its mana cost", and turn it face up any time you have priority by
//! paying its disguise cost (CR 702.168d). Riftburst Hellion ("Disguise {4}{R/G}{R/G}") prints it.
//!
//! A card with disguise lists `way(cost, text)` among its `ways`. Casting it face down and turning it face up are the
//! rules' (CR 708, 116.2b): the way casts the card as `intrinsic::DISGUISED` (ward {2}) for {3}
//! (`cantrip_api::face_down_way`), and any face-down permanent that would have disguise if it were face up - cast face down,
//! manifested or cloaked (CR 701.40d, 701.58d) - may be turned face up for this cost as a special action
//! (`intrinsic::TURN_UP_DISGUISE_TEXT`).
//!
//! ```ignore
//! ways: &[disguise::way(Cost::mana(mana("{1}{U}")), "Disguise {1}{U}")],
//! ```

use cantrip_api::*;

/// Disguise `cost`: cast face down with ward {2} for {3} from wherever the card could be cast (CR 702.168b), turned face up
/// for `cost` (CR 702.168d). `text` is the keyword as printed ("Disguise {1}{U}").
pub const fn way(cost: Cost, text: &'static str) -> WayDef {
    face_down_way(WayKind::Disguise, cost, text)
}
