use cantrip_api::*;
use cantrip_mechanics::effects;
use cantrip_oracle as oracle;

/// "Each opponent loses X life. You gain life equal to the life lost this way" (CR 107.3, 119.3, 608.2c).
fn drain_x(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    effects::each_opponent_loses_life_you_gain_life_lost(g, item.controller, item.x as i32);
    Ok(())
}

pub static CARD: CardDef = CardDef { spell: spell(&[], drain_x), ..oracle::EXSANGUINATE };
