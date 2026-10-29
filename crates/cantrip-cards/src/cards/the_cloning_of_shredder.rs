use cantrip_api::*;
use cantrip_mechanics::select::{cards, exactly};
use cantrip_mechanics::{choose, targets};
use cantrip_oracle as oracle;

/// "Create a token that's a copy of [the card], except it isn't legendary and it's a Mutant in addition to its other
/// types" (CR 707.2, 707.9b, 111.1): the copiable values of the exiled card, with both exceptions applied, so a copy of the
/// token has them too.
fn mutant_copy_of(g: &mut dyn Game, controller: PlayerId, card: ObjectId) -> Result<(), Suspend> {
    let values = g.copiable_values(card).nonlegendary().with_subtype(Subtype::Mutant);
    g.create_token_copies_of(values, controller, 1)?;
    Ok(())
}

/// I: "Exile target creature card from your graveyard. Create a token that's a copy of it, except it isn't legendary and
/// it's a Mutant in addition to its other types." The card is exiled with this Saga (CR 607.2a), for II and III. If the
/// target is illegal, nothing is exiled or copied (CR 608.2b); a card that did not reach exile is not copied (CR 608.2c).
fn exile_and_copy(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(card) = targets::object(g, item, 0) else { return Ok(()) };
    let exiled = g.exile_with(&[card], g.linked(item), false);
    let Some(&exiled) = exiled.first() else { return Ok(()) };
    g.end_sentence();
    mutant_copy_of(g, item.controller, exiled)
}

/// II and III: "Create a token that's a copy of a card exiled with this Saga, except it isn't legendary and it's a Mutant
/// in addition to its other types." The card is chosen as this resolves (CR 608.2d); the choice is asked even of a lone card (more are exiled with it if chapter I triggers again), and with
/// none exiled nothing happens.
fn copy_an_exiled_card(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let exiled = g.exiled_with(g.linked(item));
    let chosen = choose::select(g, item.controller, item.object, Purpose::CopyWhich, exiled, cards(exactly(1)))?.first().copied();
    let Some(chosen) = chosen else { return Ok(()) };
    mutant_copy_of(g, item.controller, chosen)
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        chapter(1, exile_and_copy, "I \u{2014} Exile target creature card from your graveyard. Create a token that's a copy of it, except it isn't legendary and it's a Mutant in addition to its other types.")
            .targets(&[targets::CREATURE_CARD_IN_YOUR_GRAVEYARD]),
        chapter(2, copy_an_exiled_card, "II, III \u{2014} Create a token that's a copy of a card exiled with this Saga, except it isn't legendary and it's a Mutant in addition to its other types."),
        chapter(3, copy_an_exiled_card, "II, III \u{2014} Create a token that's a copy of a card exiled with this Saga, except it isn't legendary and it's a Mutant in addition to its other types."),
    ],
    ..oracle::THE_CLONING_OF_SHREDDER
};
