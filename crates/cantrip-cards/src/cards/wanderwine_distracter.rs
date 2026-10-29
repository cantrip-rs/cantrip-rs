use cantrip_api::*;
use cantrip_mechanics::triggers::THIS_BECOMES_TAPPED;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(THIS_BECOMES_TAPPED, effects::target_creature_gets::<-3, 0>, "Whenever this creature becomes tapped, target creature an opponent controls gets -3/-0 until end of turn.")
        .targets(&[targets::CREATURE_AN_OPPONENT_CONTROLS])],
    ..oracle::WANDERWINE_DISTRACTER
};
