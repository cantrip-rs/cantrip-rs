use cantrip_api::*;
use cantrip_mechanics::effects;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[ability(cost("{R}"), effects::this_gets::<2, 0>, "{R}: This creature gets +2/+0 until end of turn. Activate only once each turn.")
        .once_each_turn()],
    madness: Some(ManaCost::ZERO),
    ..oracle::BLAZING_ROOTWALLA
};
