use cantrip_api::*;
use cantrip_mechanics::{copy, history, targets};
use cantrip_oracle as oracle;

/// "Target creature card from a graveyard that was put there this turn" (CR 115.1, 400.7): a creature card in any player's
/// graveyard that the move into it, this turn, made: destroyed, sacrificed, discarded, milled or countered, from anywhere (a card that
/// has left and come back is another object).
fn creature_card_put_into_a_graveyard_this_turn(q: &dyn Query, asker: &Asker, t: Target) -> bool {
    (targets::CREATURE_CARD_IN_A_GRAVEYARD.filter)(q, asker, t)
        && matches!(t, Target::Object(id) if history::arrived_this_turn(q, id))
}

/// "Exile target creature card from a graveyard that was put there this turn. Create a token that's a copy of it, except it's a
/// Nightmare in addition to its other types. Then exile all other Nightmare tokens you control." (CR 707.2, 707.9b, 701.13): the
/// copy is of the card as it was printed, not of what it was before it was put into the graveyard (the rulings). A card that
/// was not exiled is not copied (CR 608.2c). The Nightmare tokens that are exiled are all of those the controller has, whichever made them, but not the one just
/// made; the "Nightmare" is a word of this card's text (CR 612).
fn exile_copy_and_then_the_other_nightmare_tokens(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(card) = targets::object(g, item, 0) else { return Ok(()) };
    let exiled = g.exile_all(&[card]);
    let Some(&exiled) = exiled.first() else { return Ok(()) };
    g.end_sentence();
    let made = copy::create_token_copy_with_subtype(g, item.object, item.controller, exiled, Subtype::Nightmare, false)?;
    g.end_sentence();
    let nightmare = g.words(item.object).subtype(Subtype::Nightmare);
    let others: Vec<ObjectId> = g
        .permanents_of(item.controller)
        .into_iter()
        .filter(|&id| g.obj(id).token && g.chars(id).has_subtype(nightmare) && !made.contains(&id))
        .collect();
    g.exile_all(&others);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[ability(
        TAP,
        exile_copy_and_then_the_other_nightmare_tokens,
        "{T}: Exile target creature card from a graveyard that was put there this turn. Create a token that's a copy of it, except it's a Nightmare in addition to its other types. Then exile all other Nightmare tokens you control.",
    )
    .targets(&[target(creature_card_put_into_a_graveyard_this_turn)])],
    ..oracle::ABYSSAL_HARVESTER
};
