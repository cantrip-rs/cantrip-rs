use cantrip_api::*;
use cantrip_mechanics::history::you_gained_life_this_turn;
use cantrip_mechanics::surveil_into_graveyard;
use cantrip_mechanics::triggers;
use cantrip_oracle as oracle;

/// Surveil 1. "If you put a card with mana value less than or equal to the amount of life you gained this turn into your
/// graveyard this way, put that card into your hand": the life is counted now, as the ability resolves (so life gained
/// with it on the stack counts), and the card is the one the surveil really put into the graveyard (CR 701.25).
fn surveil_and_take(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let you = item.controller;
    let binned = surveil_into_graveyard(g, you, item.object, 1)?;
    let gained = g.history().life_gained(you);
    let taken: Vec<ObjectId> = binned.into_iter().filter(|&card| i64::from(g.mana_value(card)) <= gained).collect();
    g.return_to_hand(&taken);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(triggers::YOUR_END_STEP, surveil_and_take, "At the beginning of your end step, if you gained life this turn, surveil 1. If you put a card with mana value less than or equal to the amount of life you gained this turn into your graveyard this way, put that card into your hand.")
        .condition(you_gained_life_this_turn)],
    ..oracle::ENLIGHTENED_CONFIDANT
};
