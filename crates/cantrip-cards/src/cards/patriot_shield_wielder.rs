use cantrip_api::*;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[ability(cost("{2}, {T}"), effects::target_creature_gets_and_gains_hexproof::<2, 0>, "{2}, {T}: Another target creature you control gets +2/+0 and gains hexproof until end of turn.")
        .targets(&[targets::OTHER_CREATURE_YOU_CONTROL])],
    ..oracle::PATRIOT_SHIELD_WIELDER
};
