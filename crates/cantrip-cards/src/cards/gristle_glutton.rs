use cantrip_api::*;
use cantrip_mechanics::{blight, discard_chosen};
use cantrip_oracle as oracle;

/// "Discard a card. If you do, draw a card" (CR 701.9, 608.2c): the draw follows a card actually discarded; nothing is asked
/// of an empty hand. Blight 1 (CR 701.68a) is paid as the cost.
fn loot(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if !discard_chosen(g, item.controller, item.object, 1)?.is_empty() {
        g.draw(item.controller, 1);
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[ability(TAP.and(blight::cost(1)), loot, "{T}, Blight 1: Discard a card. If you do, draw a card.")],
    ..oracle::GRISTLE_GLUTTON
};
