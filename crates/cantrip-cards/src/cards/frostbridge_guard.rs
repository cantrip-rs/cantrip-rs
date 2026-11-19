use cantrip_api::*;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

/// "{2}{W}, {T}: Tap target creature." (CR 701.26a, 602.2): any creature, its controller's or an opponent's, tapped or not; the
/// {T} in the cost is why the Guard cannot use it the turn it came under its controller's control (CR 302.6).
pub static CARD: CardDef = CardDef {
    abilities: &[ability(cost("{2}{W}").and(TAP), effects::tap_targets, "{2}{W}, {T}: Tap target creature.").targets(&[targets::CREATURE])],
    ..oracle::FROSTBRIDGE_GUARD
};
