use cantrip_api::*;
use cantrip_mechanics::triggers::THIS_ENTERS;
use cantrip_mechanics::{effects, offspring, targets};
use cantrip_oracle as oracle;

/// "Offspring {1}{U}" and "When this creature enters, tap up to one target creature and put a stun counter on it" (CR 702.175a,
/// 701.26a, 122.1d). The 1/1 token copy has the enters ability too (CR 707.2), so a paid offspring stuns twice.
pub static CARD: CardDef = CardDef {
    kicker: Some(offspring::kicker(mana("{1}{U}"))),
    abilities: &[
        offspring::trigger("Offspring {1}{U} (You may pay an additional {1}{U} as you cast this spell. If you do, when this creature enters, create a 1/1 token copy of it.)"),
        triggered(THIS_ENTERS, effects::tap_and_stun::<1>, "When this creature enters, tap up to one target creature and put a stun counter on it.")
            .targets(&[up_to_one(targets::CREATURE.filter)]),
    ],
    ..oracle::SPLASH_LASHER
};
