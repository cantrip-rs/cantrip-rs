use cantrip_api::*;
use cantrip_mechanics::{effects, triggers};
use cantrip_oracle as oracle;

/// "Surveil 1. Then if there are seven or more cards in your graveyard, sacrifice this artifact, it deals 2 damage to
/// each opponent, and you gain 2 life." The count is taken as the ability resolves, after the surveil (CR 701.25), and
/// is not an intervening "if": the trigger always happens. Each part is done whether or not the others could be: if
/// the artifact has already left the battlefield the sacrifice does nothing, and it still deals the damage as it last
/// existed (CR 113.7a, 608.2h).
fn surveil_then_burst(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let you = item.controller;
    effects::surveil(g, item, 1)?;
    if cantrip_mechanics::threshold(g, you) {
        g.sacrifice(you, &[item.object]);
        effects::damage_each_opponent_and_gain_life(g, item, 2)?;
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(triggers::YOUR_UPKEEP, surveil_then_burst, "At the beginning of your upkeep, surveil 1. Then if there are seven or more cards in your graveyard, sacrifice this artifact, it deals 2 damage to each opponent, and you gain 2 life.")],
    ..oracle::EYE_OF_JACE
};
