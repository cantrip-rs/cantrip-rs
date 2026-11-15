use cantrip_api::*;
use cantrip_mechanics::{effects, triggers};
use cantrip_oracle as oracle;

/// "Each opponent loses 2 life. You gain 2 life and draw a card." (CR 119.3, 121.1), in the order written (CR 608.2c).
fn each_opponent_loses_2_life_then_you_gain_2_life_and_draw(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    effects::each_opponent_loses_2_life(g, item)?;
    effects::gain_life_then_draw::<2, 1>(g, item)
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        triggers::THIS_ENTERS_OR_ATTACKS,
        each_opponent_loses_2_life_then_you_gain_2_life_and_draw,
        "Whenever this creature enters or attacks, each opponent loses 2 life. You gain 2 life and draw a card.",
    )],
    ..oracle::SHROUDSTOMPER
};
