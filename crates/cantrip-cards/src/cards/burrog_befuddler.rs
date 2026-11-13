use cantrip_api::*;
use cantrip_mechanics::triggers::THIS_ENTERS;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

// Flash is the core's, read from the card's JSON.
pub static CARD: CardDef = CardDef {
    abilities: &[triggered(THIS_ENTERS, effects::target_creature_gets::<-1, 0>, "When this creature enters, target creature an opponent controls gets -1/-0 until end of turn.")
        .targets(&[targets::CREATURE_AN_OPPONENT_CONTROLS])],
    ..oracle::BURROG_BEFUDDLER
};
