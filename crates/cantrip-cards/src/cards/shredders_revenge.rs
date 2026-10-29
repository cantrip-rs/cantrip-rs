use cantrip_api::*;
use cantrip_mechanics::{discard_chosen, targets};
use cantrip_oracle as oracle;

/// Mode 1: "Target player discards two cards." (CR 701.9): their choice, discarded together; nothing happens if the target
/// is no longer legal (CR 608.2b).
fn discard_two(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(player) = targets::player(g, item, 0) else { return Ok(()) };
    discard_chosen(g, player, item.object, 2)?;
    Ok(())
}

/// Mode 2: "Target player draws two cards and loses 2 life." (CR 121.1, 119.3): both happen to the targeted player if it is still
/// a legal target (CR 608.2b), and the life is lost even when the library was empty (CR 121.4).
fn draw_two_and_lose_2(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if let Some(player) = targets::player(g, item, 0) {
        g.draw(player, 2);
        g.lose_life(player, 2);
    }
    Ok(())
}

static MODAL: ModalDef = modes(&[
    mode(discard_two, "Target player discards two cards.").targets(&[targets::PLAYER]),
    mode(draw_two_and_lose_2, "Target player draws two cards and loses 2 life.").targets(&[targets::PLAYER]),
]);

pub static CARD: CardDef = CardDef { spell: Some(SpellDef::MODAL), modal: Some(&MODAL), ..oracle::SHREDDERS_REVENGE };
