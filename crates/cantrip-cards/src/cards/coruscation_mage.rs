use cantrip_api::*;
use cantrip_mechanics::{effects, offspring, triggers};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    kicker: Some(offspring::kicker(mana("{2}"))),
    abilities: &[
        offspring::trigger("Offspring {2} (You may pay an additional {2} as you cast this spell. If you do, when this creature enters, create a 1/1 token copy of it.)"),
        triggered(triggers::NONCREATURE_SPELL_YOU_CAST, effects::damage_each_opponent_n::<1>, "Whenever you cast a noncreature spell, this creature deals 1 damage to each opponent."),
    ],
    ..oracle::CORUSCATION_MAGE
};
