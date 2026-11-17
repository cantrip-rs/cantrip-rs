use cantrip_api::*;
use cantrip_mechanics::{disguise, effects, mana, targets, triggers};
use cantrip_oracle as oracle;

/// "Target opponent loses 3 life and you gain 3 life." (CR 119.3)
fn target_opponent_loses_3_life_you_gain_3(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    effects::target_player_loses_life_you_gain_life(g, item, 3)
}

pub static CARD: CardDef = CardDef {
    enters: Some(mana::enters_tapped),
    abilities: &[triggered(
        triggers::THIS_TURNED_FACE_UP,
        target_opponent_loses_3_life_you_gain_3,
        "When this creature is turned face up, target opponent loses 3 life and you gain 3 life.",
    )
    .targets(&[targets::OPPONENT])],
    ways: &[disguise::way(Cost::mana(mana("{4}{B}{B}")), "Disguise {4}{B}{B}")],
    ..oracle::ALLEY_ASSAILANT
};
