use cantrip_api::*;
use cantrip_mechanics::{effects, offspring, targets, triggers::THIS_ENTERS};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    kicker: Some(offspring::kicker(mana("{1}"))),
    abilities: &[
        offspring::trigger("Offspring {1} (You may pay an additional {1} as you cast this spell. If you do, when this creature enters, create a 1/1 token copy of it.)"),
        triggered(THIS_ENTERS, effects::target_creature_gets::<1, 1>, "When this creature enters, target creature you control gets +1/+1 until end of turn.")
            .targets(&[targets::CREATURE_YOU_CONTROL]),
    ],
    ..oracle::INTREPID_RABBIT
};
