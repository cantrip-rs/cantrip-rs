use cantrip_api::*;
use cantrip_mechanics::{dredge::dredge, regenerate};
use cantrip_oracle as oracle;

/// "This creature enters with a +1/+1 counter on it for each creature card in your graveyard" (CR 614.1c, 122.6): counted
/// as it enters, in the graveyard of the player it enters under. It is not on the battlefield yet, so a Troll put onto the
/// battlefield from that graveyard counts itself (its ruling).
fn enters_with_counters(g: &mut dyn Game, entering: &mut Entering) -> Result<(), Suspend> {
    let creatures = cantrip_mechanics::creature_cards_in_graveyard(g, entering.controller);
    entering.counters[CounterKind::PlusOne as usize] += creatures.min(i16::MAX as usize) as i16;
    Ok(())
}

/// "Regenerate this creature" (CR 701.19a).
fn regenerate_this(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    regenerate::this(g, item);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    enters: Some(enters_with_counters),
    abilities: &[ability(
        cost("{1}").and(remove_counters(CounterKind::PlusOne, 1)),
        regenerate_this,
        "{1}, Remove a +1/+1 counter from this creature: Regenerate this creature.",
    )],
    // "Dredge 6 (If you would draw a card, you may mill six cards instead. If you do, return this card from your graveyard
    // to your hand.)"
    replacements: &[dredge::<6>()],
    ..oracle::GOLGARI_GRAVE_TROLL
};
