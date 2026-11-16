use cantrip_api::*;
use cantrip_mechanics::{forage, triggers};
use cantrip_oracle as oracle;

/// "When this creature enters, you may forage. If you do, draw a card." (CR 701.61a, 608.2d): forage is a cost paid as the
/// ability resolves, so the controller is asked only about a way they can pay, and draws only if they did.
fn forage_to_draw(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if forage::may_forage(g, item)? {
        g.draw(item.controller, 1);
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        triggers::THIS_ENTERS,
        forage_to_draw,
        "When this creature enters, you may forage. If you do, draw a card. (To forage, exile three cards from your graveyard or sacrifice a Food.)",
    )],
    ..oracle::TREETOP_SENTRIES
};
