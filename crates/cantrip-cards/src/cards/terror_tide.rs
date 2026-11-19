use cantrip_api::*;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

/// "All creatures get -X/-X until end of turn, where X is the number of permanent cards in your graveyard" (CR 110.4a, 611.2c):
/// X is counted as the spell resolves, so the Terror Tide itself (a sorcery card, and still on the stack) is not one, and the
/// creatures that get it are those on the battlefield then.
fn all_creatures_get_minus_x_x(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let x = targets::permanent_cards_in_graveyard(g, item.controller) as i32;
    effects::pump_all_creatures(g, item.object, -x, -x);
    Ok(())
}

pub static CARD: CardDef = CardDef { spell: spell(&[], all_creatures_get_minus_x_x), ..oracle::TERROR_TIDE };
