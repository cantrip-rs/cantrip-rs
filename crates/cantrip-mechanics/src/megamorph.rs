//! Megamorph [cost] (CR 702.37b): morph (CR 702.37a), and "As this permanent is turned face up, put a +1/+1 counter on it if
//! its megamorph cost was paid to turn it face up." A megamorph cost is a morph cost. Den Protector (Megamorph {1}{G}) and
//! Hidden Dragonslayer (Megamorph {2}{W}) print it.
//!
//! A card with megamorph lists `way(cost, text)` among its `ways`. The counter is put by the rules' special action as it
//! turns the permanent face up for this cost (CR 708.11, `intrinsic::TURN_UP_MORPH_TEXT`), and only then: a manifested or
//! cloaked megamorph card turned face up for its mana cost (CR 701.40b-c, 701.58b-c) gets none.
//!
//! ```ignore
//! ways: &[megamorph::way(Cost::mana(mana("{2}{G}")), "Megamorph {2}{G}")],
//! ```

use cantrip_api::*;

/// Megamorph `cost`: cast face down for {3} (CR 702.37b-c), turned face up for `cost` with a +1/+1 counter (CR 702.37b,
/// 702.37e). `text` is the keyword as printed ("Megamorph {2}{G}").
pub const fn way(cost: Cost, text: &'static str) -> WayDef {
    face_down_way(WayKind::Megamorph, cost, text)
}
