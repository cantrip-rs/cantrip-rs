use cantrip_api::*;
use cantrip_mechanics::{effects, triggers::EACH_END_STEP};
use cantrip_oracle as oracle;

/// "Each opponent loses life equal to the life that player lost this turn. You gain life equal to the life lost this way." (CR 119.3, 608.2c, 608.2h).
fn drain(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let lost = effects::each_opponent_loses_life_lost_this_turn(g, item.controller);
    if lost > 0 {
        g.life_changes_together(&[LifeChange { player: item.controller, magnitude: lost, negative: false }]);
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        EACH_END_STEP,
        drain,
        "At the beginning of each end step, each opponent loses life equal to the life that player lost this turn. You gain life equal to the life lost this way. (Damage causes loss of life.)",
    )],
    ..oracle::GUL_DUKAT_COLONIAL_PREFECT
};
