use cantrip_api::*;
use cantrip_mechanics::triggers::THIS_ENTERS;
use cantrip_mechanics::{effects, scry, targets};
use cantrip_oracle as oracle;

/// Mode 1: "Put a +1/+1 counter on this creature for each creature that left the battlefield under your control this
/// turn." Counted from the event history as the ability resolves: every move off the battlefield this turn of an object
/// that was a creature as it left (last known information, so a token that has ceased to exist counts, CR 113.7a) and
/// that `you` controlled as it left. Each object that left is counted once, so a card that left, returned and left again
/// counts twice (CR 400.7).
fn counters(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let you = item.controller;
    let left = g
        .history()
        .select(EventQuery::kind(EventMask::ZONE_CHANGE).from(ZoneKind::Battlefield).this_turn())
        .filter(|r| {
            matches!(r.event, Event::ZoneChange { old, .. }
                if g.state().last_known(old).is_some_and(|l| l.types.has(CardType::Creature))
                    && g.obj(old).controller == you)
        })
        .count();
    g.add_counters(item.object, CounterKind::PlusOne, i64::from(left as i16));
    Ok(())
}

/// Mode 2: "You gain 2 life and scry 2." (CR 119.1, 701.22.)
fn life_and_scry(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    effects::gain_life(g, item, 2)?;
    scry(g, item.controller, item.object, 2)
}

static MODAL: ModalDef = modes(&[
    mode(counters, "Put a +1/+1 counter on this creature for each creature that left the battlefield under your control this turn."),
    mode(life_and_scry, "You gain 2 life and scry 2."),
    mode(effects::exile_first_target_players_graveyard, "Exile target player's graveyard.").targets(&[targets::PLAYER]),
]);

pub static CARD: CardDef = CardDef {
    abilities: &[modal_triggered(THIS_ENTERS, &MODAL, "When this creature enters, choose one \u{2014} Put a +1/+1 counter on this creature for each creature that left the battlefield under your control this turn; or you gain 2 life and scry 2; or exile target player's graveyard.")],
    ..oracle::KUTZILS_FLANKER
};
