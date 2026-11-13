use cantrip_api::*;
use cantrip_mechanics::replacements::{instead_of_drawing, you_would_draw_a_card};
use cantrip_mechanics::{choose, select};
use cantrip_oracle as oracle;

/// "... instead put a card you own from outside the game into your hand" (CR 400.11b, 614.6): the player puts one of the cards
/// they own outside the game into their hand if there is one.
fn from_outside_the_game(g: &mut dyn Game, asker: &Asker, _: ObjectId, _: &Proposal, _: &Proposal) {
    let _ = put_card_from_outside_the_game(g, asker);
}

fn put_card_from_outside_the_game(g: &mut dyn Game, asker: &Asker) -> Result<(), Suspend> {
    let outside = g.outside_the_game(asker.controller);
    let chosen = choose::select(
        g,
        asker.controller,
        asker.source,
        Purpose::PutIntoHand,
        outside,
        select::Selection::new(select::Count::AsManyAsCan(1)),
    )?;
    g.bring_into_hand(&chosen);
    Ok(())
}

/// A replacement effect about the player, lasting the turn and used up by the one draw it replaces (CR 614.1a, 611.2).
fn next_time_would_draw_card(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    g.add_player_effect(
        item.controller,
        item.object,
        EffectKind::ReplacementRule { card: item.ability.card, index: 0 },
        Duration::EndOfTurn,
    );
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[ability(
        cost("{5}").and(TAP).and(EXILE_THIS),
        next_time_would_draw_card,
        "{5}, {T}, Exile this artifact: The next time you would draw a card this turn, instead put a card you own from outside the game into your hand.",
    )],
    granted_replacements: &[granted_replacement(
        ReplacementKind::Draw,
        |q, asker, _, proposal| you_would_draw_a_card(q, asker, proposal),
        instead_of_drawing_apply,
        "instead put a card you own from outside the game into your hand",
    )
    .once()
    .then(from_outside_the_game)
    .text("The next time you would draw a card this turn, instead put a card you own from outside the game into your hand.")],
    ..oracle::RING_OF_MAR_F
};

fn instead_of_drawing_apply(q: &dyn Query, asker: &Asker, _: ObjectId, proposal: &mut Proposal) {
    instead_of_drawing(q, asker, proposal);
}
