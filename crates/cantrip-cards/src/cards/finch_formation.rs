use cantrip_api::*;
use cantrip_mechanics::offspring;
use cantrip_mechanics::triggers::THIS_ENTERS;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    kicker: Some(offspring::kicker(mana("{3}"))),
    abilities: &[
        offspring::trigger("Offspring {3} (You may pay an additional {3} as you cast this spell. If you do, when this creature enters, create a 1/1 token copy of it.)"),
        triggered(THIS_ENTERS, effects::first_target_gains_flying, "When this creature enters, target creature you control gains flying until end of turn.")
            .targets(&[targets::CREATURE_YOU_CONTROL]),
    ],
    ..oracle::FINCH_FORMATION
};
