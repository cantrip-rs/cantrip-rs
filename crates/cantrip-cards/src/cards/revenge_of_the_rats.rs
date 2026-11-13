use cantrip_api::*;
use cantrip_mechanics::{creature_cards_in_graveyard, flashback, tokens};
use cantrip_oracle as oracle;

/// "Create a tapped 1/1 black Rat creature token for each creature card in your graveyard" (CR 111.1, 404.1): counted as the
/// spell resolves, and a spell cast with flashback is still in the stack, not in the graveyard, so it is not counted.
fn effect(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let count = creature_cards_in_graveyard(g, item.controller) as u32;
    // A token the spell's own text makes: it has the words of that text (CR 612.2a), so `create_by_rule` is not for it.
    let rat = g.card_id(&tokens::RAT);
    g.create_tokens(rat, item.controller, count, true)?;
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[], effect),
    ways: &[flashback::way(cost("{2}{B}{B}"), "Flashback {2}{B}{B}")],
    ..oracle::REVENGE_OF_THE_RATS
};

pub static EXTRA: &[&CardDef] = &[&tokens::RAT];
