use cantrip_api::*;
use cantrip_mechanics::effects;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[ability(cost("{1}{G}"), effects::this_gets::<2, 2>, "{1}{G}: This creature gets +2/+2 until end of turn. Activate only once each turn.")
        .once_each_turn()],
    madness: Some(ManaCost::ZERO),
    ..oracle::BASKING_ROOTWALLA
};
