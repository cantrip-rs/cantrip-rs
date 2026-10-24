use cantrip_api::*;
use cantrip_mechanics::triggers::THIS_ENTERS;
use cantrip_mechanics::{choose, select, tokens};
use cantrip_oracle as oracle;

/// "When this creature enters, you may sacrifice a land. If you do, create two tapped Heartwood tokens." (CR 603.6a,
/// 701.21a, 111.10x): the sacrifice is chosen as the trigger resolves and may be declined (CR 608.2d), and only a land
/// may be sacrificed. The two Heartwood tokens are created together, tapped, under the trigger's controller.
fn may_trade_a_land(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let you = item.controller;
    let permanents = g.permanents_of(you);
    if !permanents.iter().any(|&id| g.chars(id).is(CardType::Land)) {
        return Ok(());
    }
    let rule = Selection::new(select::up_to(1)).with(select::LAND).called("land");
    let chosen = choose::select(g, you, item.object, Purpose::Sacrifice, permanents, rule)?;
    if chosen.is_empty() {
        return Ok(());
    }
    // "If you do": the tokens follow only a sacrifice that happened (CR 608.2c).
    if g.sacrifice(&chosen).is_empty() {
        return Ok(());
    }
    let heartwood = g.card_id(&tokens::HEARTWOOD);
    g.create_tokens(heartwood, you, 2, true)?;
    Ok(())
}

/// "Tap two untapped artifacts you control": a cost, not the {T} symbol of those artifacts (CR 118.3, 107.5), so an
/// artifact creature that entered this turn may be tapped for it. Chosen as the cost is paid, and tapped then,
/// among the payer's own permanents (the engine offers only those, so "you control" needs no test here).
fn untapped_artifact(q: &dyn Query, _: &Asker, id: ObjectId) -> bool {
    !q.obj(id).tapped && q.chars(id).is(CardType::Artifact)
}

/// "Put two +1/+1 counters on this creature." Nothing if it has left the battlefield (CR 400.7).
fn two_counters(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    g.add_counters(item.object, CounterKind::PlusOne, 2);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(THIS_ENTERS, may_trade_a_land, "When this creature enters, you may sacrifice a land. If you do, create two tapped Heartwood tokens."),
        ability(
            Cost::tap_untapped(2, untapped_artifact),
            two_counters,
            "Tap two untapped artifacts you control: Put two +1/+1 counters on this creature.",
        ),
    ],
    ..oracle::TENURED_TETHERMAGE
};

pub static EXTRA: &[&CardDef] = &[&tokens::HEARTWOOD];
