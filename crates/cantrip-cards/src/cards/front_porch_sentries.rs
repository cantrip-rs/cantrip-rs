use cantrip_api::*;
use cantrip_mechanics::{effects, targets, triggers};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(triggers::THIS_DIES, effects::target_creature_gets::<-1, -1>, "When this creature dies, target creature an opponent controls gets -1/-1 until end of turn.")
        .targets(&[targets::CREATURE_AN_OPPONENT_CONTROLS])],
    ..oracle::FRONT_PORCH_SENTRIES
};
