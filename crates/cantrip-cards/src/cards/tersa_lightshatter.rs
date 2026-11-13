use cantrip_api::*;
use cantrip_mechanics::hand;
use cantrip_mechanics::triggers::THIS_ENTERS;
use cantrip_oracle as oracle;

/// When this enters, discard up to two cards, then draw that many cards. The discards happen together (CR 701.9),
/// before the draw.
fn discard_and_draw(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    hand::discard_up_to_then_draw(g, item.controller, item.object, 2)
}

/// Exile a card at random from your graveyard. You may play that card this turn.
fn exile_and_play(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let player = item.controller;
    let cards: Vec<ObjectId> = g.state().zone(Zone::graveyard(player)).to_vec();
    let Some(card) = cantrip_mechanics::library::random_of(g, &cards) else { return Ok(()) };
    let exiled = g.exile(card);
    if cantrip_mechanics::library::is_still_exiled(g, exiled) {
        g.allow_play(exiled, player, PlayGrant { lands: true, expiry: Expiry::EndOfTurn, ..PlayGrant::CAST });
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(THIS_ENTERS, discard_and_draw, "When Tersa Lightshatter enters, discard up to two cards, then draw that many cards."),
        triggered(cantrip_mechanics::triggers::THIS_ATTACKS, exile_and_play, "Whenever Tersa Lightshatter attacks, if there are seven or more cards in your graveyard, exile a card at random from your graveyard. You may play that card this turn.")
            .condition(cantrip_mechanics::you_have_threshold),
    ],
    ..oracle::TERSA_LIGHTSHATTER
};
