use cantrip_api::*;
use cantrip_mechanics::lands;
use cantrip_mechanics::statics::{have_double_strike, other_of_subtype_you_control};
use cantrip_oracle as oracle;

/// "Other Giants you control have double strike." (CR 702.4, 613.1f, layer 6): every other permanent its controller controls
/// that is a Giant as the earlier layers have it, the type read through the words of this card's text (CR 612.2). The
/// Aggressor's own double strike is its printed keyword.
pub static CARD: CardDef = CardDef {
    statics: &[static_ability(
        Layer::Ability,
        other_of_subtype_you_control::<{ lands::sub(Subtype::Giant) }>,
        have_double_strike,
        "Other Giants you control have double strike.",
    )],
    ..oracle::BOLDWYR_AGGRESSOR
};
