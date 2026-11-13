use cantrip_api::*;
use cantrip_mechanics::triggers::{YOU_CYCLE_THIS_CARD, YOU_DISCARD, cycling};
use cantrip_mechanics::effects;
use cantrip_oracle as oracle;

/// "This creature deals that much damage to each opponent" (CR 120.3, 603.2c): as many as the cards of the batch this ability saw
/// discarded together, counted even for a card that has since left the graveyard.
fn that_much_damage_to_each_opponent(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let many = g.event_objects(item).len() as u32;
    effects::damage_each_opponent(g, item, many);
    Ok(())
}

/// "When you cycle this card, it deals 1 damage to each opponent": the card, in the graveyard, deals it (CR 113.7a).
fn one_damage_to_each_opponent(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    effects::damage_each_opponent(g, item, 1);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(YOU_DISCARD, that_much_damage_to_each_opponent, "Whenever you discard one or more cards, this creature deals that much damage to each opponent.").once_per_batch(),
        cycling(mana("{1}{R}"), "Cycling {1}{R} ({1}{R}, Discard this card: Draw a card.)"),
        triggered(YOU_CYCLE_THIS_CARD, one_damage_to_each_opponent, "When you cycle this card, it deals 1 damage to each opponent."),
    ],
    ..oracle::MAGMAKIN_ARTILLERIST
};
