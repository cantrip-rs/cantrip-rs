use cantrip_api::*;
use cantrip_mechanics::history::committed_a_crime_this_turn;
use cantrip_mechanics::triggers::BEGINNING_OF_YOUR_COMBAT;
use cantrip_mechanics::{discard_chosen, effects, targets, tokens};
use cantrip_oracle as oracle;

/// "Oko becomes a copy of up to one target creature you control until end of turn, except he has hexproof." (CR 707.2, 707.9b,
/// 611.2a, 613): the target's copiable values with hexproof added, until end of turn, so he is that creature and no longer a
/// planeswalker (no loyalty abilities, damage to him is damage to a creature), keeping his loyalty counters, and is neither
/// entering nor leaving (the rulings). With no target, or one that is no longer legal, nothing happens (CR 608.2b).
fn become_a_copy_with_hexproof(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(creature) = targets::object(g, item, 0) else { return Ok(()) };
    let values = g.copiable_values(creature).with_keyword(Keyword::Hexproof);
    g.become_copy(item.object, item.object, values, Duration::EndOfTurn);
    Ok(())
}

/// "+1: Draw two cards. If you've committed a crime this turn, discard a card. Otherwise, discard two cards." (CR 701.9, 700.13): the
/// crime is read as the discard is chosen, after the draw, and commit one earlier this turn only (a crime is committed as the
/// spell is cast or ability put on the stack that targets an opponent or their things, so Oko's own targetless ability is none).
fn draw_two_then_discard(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    effects::draw(g, item, 2)?;
    g.end_sentence();
    let count = if committed_a_crime_this_turn(g.history(), item.controller) { 1 } else { 2 };
    discard_chosen(g, item.controller, item.object, count)?;
    Ok(())
}

/// "\u{2212}5: For each other nonland permanent you control, create a token that's a copy of that permanent." (CR 707.2, 101.4,
/// 603.2c): the permanents are found once as the ability resolves (CR 608.2c), before any token exists, each copied as it is
/// (a token's original characteristics, the rulings), and the tokens enter together so each sees the others enter.
fn copy_each_other_nonland_permanent(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let you = item.controller;
    let permanents: Vec<ObjectId> = g.permanents_of(you).into_iter().filter(|&p| p != item.object && !g.chars(p).is(CardType::Land)).collect();
    let copies: Vec<(Copiable, PlayerId)> = permanents.into_iter().map(|p| (g.copiable_values(p), you)).collect();
    g.create_token_copies_together(&copies)?;
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            BEGINNING_OF_YOUR_COMBAT,
            become_a_copy_with_hexproof,
            "At the beginning of combat on your turn, Oko becomes a copy of up to one target creature you control until end of turn, except he has hexproof.",
        )
        .targets(&[targets::CREATURE_YOU_CONTROL.optional()]),
        ability(loyalty(1), draw_two_then_discard, "+1: Draw two cards. If you've committed a crime this turn, discard a card. Otherwise, discard two cards."),
        ability(loyalty(-1), tokens::create_elks::<1>, "\u{2212}1: Create a 3/3 green Elk creature token."),
        ability(loyalty(-5), copy_each_other_nonland_permanent, "\u{2212}5: For each other nonland permanent you control, create a token that's a copy of that permanent."),
    ],
    ..oracle::OKO_THE_RINGLEADER
};

pub static EXTRA: &[&CardDef] = &[&tokens::ELK];
