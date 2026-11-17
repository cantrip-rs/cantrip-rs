use cantrip_api::*;
use cantrip_mechanics::triggers::THIS_ENTERS;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(THIS_ENTERS, effects::target_creature_gets::<-2, -2>, "When this creature enters, target creature an opponent controls gets -2/-2 until end of turn.")
        .targets(&[targets::CREATURE_AN_OPPONENT_CONTROLS])],
    ..oracle::AMBUSH_GIGAPEDE
};
