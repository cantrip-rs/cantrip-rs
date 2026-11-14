use cantrip_api::*;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

/// "{2}{G}: Put a +1/+1 counter on target creature you control. Activate only as a sorcery." (CR 602.5d, 122.1a): the target is
/// chosen as the ability is activated and must still be legal as it resolves (CR 608.2b). Reach and Deathtouch come from the
/// card's keywords (CR 702.17, 702.2).
pub static CARD: CardDef = CardDef {
    abilities: &[ability(cost("{2}{G}"), effects::plus_one_counter_on_target, "{2}{G}: Put a +1/+1 counter on target creature you control. Activate only as a sorcery.")
        .sorcery_speed()
        .targets(&[targets::CREATURE_YOU_CONTROL])],
    ..oracle::TREETOP_SNARESPINNER
};
