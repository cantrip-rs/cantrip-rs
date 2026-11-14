use cantrip_api::*;
use cantrip_mechanics::{each_discards_a_card, triggers};
use cantrip_oracle as oracle;

/// "Each player discards a card. If you discarded a card this way, draw a card" (CR 701.9, 101.4): everyone chooses in turn
/// order from the active player and the cards go together (nothing is asked of an empty hand); the draw follows only when one
/// of the cards discarded was the controller's, however a replacement sent it on.
fn discard_then_draw_if_you_discarded(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let players: Vec<PlayerId> = g.state().apnap().collect();
    let discarded = each_discards_a_card(g, &players, item.object)?;
    if discarded.iter().any(|&card| g.state().obj(card).owner == item.controller) {
        g.draw(item.controller, 1);
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        triggers::THIS_ENTERS,
        discard_then_draw_if_you_discarded,
        "When this creature enters, each player discards a card. If you discarded a card this way, draw a card.",
    )],
    ..oracle::FANATIC_OF_THE_HARROWING
};
