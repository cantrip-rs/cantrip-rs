use cantrip_api::*;
use cantrip_mechanics::triggers::THIS_ENTERS_OR_ATTACKS;
use cantrip_mechanics::{library, targets};
use cantrip_oracle as oracle;

/// "A historic permanent card" (CR 700.6, 110.4a): a card with a permanent type that is an artifact, legendary or Saga.
fn is_historic_permanent_card(q: &dyn Query, card: ObjectId) -> bool {
    let c = q.chars(card);
    targets::is_permanent_card(&c) && (c.is(CardType::Artifact) || c.supertypes.legendary || c.has_subtype(Subtype::Saga))
}

/// "Whenever Jhoira enters or attacks, target opponent reveals cards from the top of their library until they reveal a
/// historic permanent card. You put that card onto the battlefield under your control and lose life equal to that
/// permanent's mana value. That player puts the rest of the revealed cards on the bottom of their library in a random
/// order."
///
/// The life lost is the mana value the permanent has on the battlefield (CR 608.2h): a copy's, or one an effect changed,
/// not the printed cost. A permanent that a replacement keeps from entering is no permanent to count, so nothing is lost;
/// with no historic permanent card in the library the whole library is revealed and put back (CR 701.20, 401.4).
fn reveal_and_take(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let you = item.controller;
    let Some(opponent) = targets::player(g, item, 0) else { return Ok(()) };
    let (found, rest) = library::reveal_until(g, opponent, 1, &is_historic_permanent_card);
    if let Some(card) = found.first().copied() {
        let permanent = g.put_onto_battlefield(card, you, false)?;
        if g.is_on_battlefield(permanent) {
            let mana_value = g.mana_value(permanent);
            g.lose_life(you, mana_value as i32);
        }
    }
    g.bottom_in_random_order(opponent, &rest);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        THIS_ENTERS_OR_ATTACKS,
        reveal_and_take,
        "Whenever Jhoira enters or attacks, target opponent reveals cards from the top of their library until they reveal a historic permanent card. You put that card onto the battlefield under your control and lose life equal to that permanent's mana value. That player puts the rest of the revealed cards on the bottom of their library in a random order.",
    )
    .targets(&[targets::OPPONENT])],
    ..oracle::JHOIRA_WEATHERLIGHT_CORSAIR
};
