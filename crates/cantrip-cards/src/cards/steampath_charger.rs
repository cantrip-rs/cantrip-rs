use cantrip_api::*;
use cantrip_mechanics::triggers::THIS_DIES;
use cantrip_mechanics::{effects, offspring, targets};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    kicker: Some(offspring::kicker(mana("{2}"))),
    abilities: &[
        offspring::trigger("Offspring {2} (You may pay an additional {2} as you cast this spell. If you do, when this creature enters, create a 1/1 token copy of it.)"),
        triggered(THIS_DIES, effects::damage_first_target::<1>, "When this creature dies, it deals 1 damage to target player.").targets(&[targets::PLAYER]),
    ],
    ..oracle::STEAMPATH_CHARGER
};
