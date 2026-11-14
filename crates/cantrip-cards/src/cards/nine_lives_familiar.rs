use cantrip_api::*;
use cantrip_mechanics::triggers::THIS_DIES;
use cantrip_mechanics::{delayed, history};
use cantrip_oracle as oracle;

/// "This creature enters with eight revival counters on it if you cast it" (CR 614.1c, 122.1, 601.2): the Familiar is the spell its
/// controller cast. Put onto the battlefield by anything else (a reanimation spell), it enters without them.
fn enters_with_eight_revival_counters_if_cast(g: &mut dyn Game, entering: &mut Entering) -> Result<(), Suspend> {
    if history::cast_of(g, entering.object).is_some_and(|cast| cast.player == entering.controller) {
        entering.counters[CounterKind::Revival as usize] += 8;
    }
    Ok(())
}

/// "If it had a revival counter on it" (CR 603.4, 603.10a): the permanent as it last was on the battlefield, as the ability
/// triggers and again as it resolves.
fn had_a_revival_counter(q: &dyn Query, asker: &Asker) -> bool {
    q.obj(asker.source).counters(CounterKind::Revival) > 0
}

/// The delayed ability: each carried card still the card the permanent became in the graveyard (CR 400.7, 603.7c) returns under the delayed ability's controller's control (CR 110.2a, 603.7e: the Familiar's controller when it died) with
/// one fewer revival counter than the permanent had when it died (CR 122.6, 603.10a): none, if it had only the one.
fn return_with_one_fewer(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let fewer = g.obj(item.object).counters(CounterKind::Revival) - 1;
    let returning: Vec<(ObjectId, PlayerId, bool)> = g
        .event_objects(item)
        .to_vec()
        .into_iter()
        .filter(|&card| is_live(g, card))
        .map(|card| (card, item.controller, false))
        .collect();
    g.put_all_onto_battlefield_with_counters(&returning, &[(CounterKind::Revival, fewer)])?;
    Ok(())
}

pub static CARD: CardDef = CardDef {
    enters: Some(enters_with_eight_revival_counters_if_cast),
    abilities: &[triggered(
        THIS_DIES,
        delayed::at_next_end_step_about_what_it_became::<0>,
        "When this creature dies, if it had a revival counter on it, return it to the battlefield with one fewer revival counter on it at the beginning of the next end step.",
    )
    .condition(had_a_revival_counter)],
    delayed: &[delayed_trigger(return_with_one_fewer, "Return it to the battlefield with one fewer revival counter on it.")],
    ..oracle::NINE_LIVES_FAMILIAR
};
