use cantrip_api::*;
use cantrip_mechanics::{disguise, effects, targets, triggers};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    ways: &[disguise::way(Cost::mana(mana("{5}{G}")), "Disguise {5}{G}")],
    abilities: &[triggered(
        triggers::THIS_TURNED_FACE_UP,
        effects::destroy_first_target,
        "When this creature is turned face up, destroy target artifact or enchantment an opponent controls.",
    )
    .targets(&[targets::ARTIFACT_OR_ENCHANTMENT_AN_OPPONENT_CONTROLS])],
    ..oracle::VENGEFUL_CREEPER
};
