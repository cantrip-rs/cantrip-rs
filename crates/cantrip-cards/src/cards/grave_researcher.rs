use cantrip_api::*;
use cantrip_mechanics::{creature_cards_in_graveyard, effects, triggers};
use cantrip_oracle as oracle;

/// "Surveil 1. Then if there are three or more creature cards in your graveyard, this creature becomes prepared." (CR 701.25, 722.3a):
/// the count is taken after the surveil, so a creature card it puts into the graveyard counts; the second sentence ends the first
/// (CR 608.2c).
fn surveil_then_prepare(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    effects::surveil(g, item, 1)?;
    g.end_sentence();
    if creature_cards_in_graveyard(g, item.controller) >= 3 {
        g.become_prepared(item.object);
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        triggers::YOUR_UPKEEP,
        surveil_then_prepare,
        "At the beginning of your upkeep, surveil 1. Then if there are three or more creature cards in your graveyard, this creature becomes prepared. (While it's prepared, you may cast a copy of its spell. Doing so unprepares it.)",
    )],
    prepare: Some(&crate::reanimate::CARD),
    ..oracle::GRAVE_RESEARCHER
};
