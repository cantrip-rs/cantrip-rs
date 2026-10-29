use cantrip_api::*;
use cantrip_mechanics::targets;
use cantrip_mechanics::triggers::THIS_ENTERS;
use cantrip_oracle as oracle;

/// "Target noncreature, nonland card from your graveyard" (CR 115.1): any card there that is neither a creature nor a land.
fn noncreature_nonland_card_in_your_graveyard(q: &dyn Query, asker: &Asker, t: Target) -> bool {
    match t {
        Target::Object(id) => {
            targets::card_in_your_graveyard(q, asker, t, &[]) && !q.chars(id).is(CardType::Creature) && !q.chars(id).is(CardType::Land)
        }
        _ => false,
    }
}

/// "Exile target noncreature, nonland card from your graveyard. Until the end of your next turn, you may cast that card"
/// (CR 601.3): the permission is for the card that actually reaches exile (CR 400.7), and is to cast it only.
fn exile_and_may_cast(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(card) = targets::object(g, item, 0) else { return Ok(()) };
    let exiled = g.exile(card);
    if g.obj(exiled).zone.is(ZoneKind::Exile) {
        g.allow_play(exiled, item.controller, PlayGrant { instead: None, lands: false, expiry: Expiry::EndOfYourNextTurn, ..PlayGrant::CAST });
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        THIS_ENTERS,
        exile_and_may_cast,
        "When this creature enters, exile target noncreature, nonland card from your graveyard. Until the end of your next turn, you may cast that card.",
    )
    .targets(&[target(noncreature_nonland_card_in_your_graveyard)])],
    ..oracle::PRACTICED_SCROLLSMITH
};
