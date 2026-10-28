use cantrip_api::*;
use cantrip_mechanics::combat::choose_attack_target;
use cantrip_mechanics::{library, triggers};
use cantrip_oracle as oracle;

/// Reveal cards from the top of your library until you reveal a creature card. Put that card onto the battlefield
/// tapped and attacking and the rest on the bottom of your library in a random order.
fn reveal(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let you = item.controller;
    let (creature, revealed) = library::reveal_until(g, you, &|q, card| q.chars(card).is(CardType::Creature));
    if creature.is_none() && revealed.is_empty() {
        return Ok(());
    }
    if let Some(card) = creature {
        let defender = choose_attack_target(g, you, item.object)?;
        match defender {
            Some(d) => {
                g.put_onto_battlefield_attacking(card, you, d, &[])?;
            }
            None => {
                g.put_onto_battlefield(card, you, true)?;
            }
        }
    }
    g.bottom_in_random_order(you, &revealed);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(triggers::THIS_ATTACKS, reveal, "Whenever Raph & Mikey attack, reveal cards from the top of your library until you reveal a creature card. Put that card onto the battlefield tapped and attacking and the rest on the bottom of your library in a random order.")],
    ..oracle::RAPH_MIKEY_TROUBLEMAKERS
};
