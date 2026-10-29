use cantrip_api::*;
use cantrip_mechanics::{delayed, effects, targets, triggers};
use cantrip_oracle as oracle;

/// "Target non-Doctor permanent card from your graveyard" (CR 115.1, 612.1): Doctor is a word of this ability's text.
fn non_doctor_permanent_in_your_graveyard(q: &dyn Query, asker: &Asker, t: Target) -> bool {
    targets::permanent_card_in_your_graveyard(q, asker, t)
        && matches!(t, Target::Object(card) if !q.chars(card).has_subtype(q.words(asker.source).subtype(Subtype::Doctor)))
}

/// "Choose up to one other target creature. When it dies this turn ..." (CR 603.7a-c): a delayed ability about
/// this exact creature, lasting this turn even if its controller changes.
fn watch_for_death(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(creature) = targets::object(g, item, 0) else { return Ok(()) };
    delayed::this_turn(g, item, creature);
    Ok(())
}

/// "... at the beginning of the next end step" (CR 603.7): set up the second delayed ability about the card the
/// dying creature became. Leaving its graveyard before then makes it a different object (CR 400.7).
fn return_at_end_step(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if let Some(card) = g.found_after_move(item.object) {
        delayed::at_next_end_step(g, item, 1, Target::Object(card));
    }
    Ok(())
}

/// "Return that card to the battlefield under its owner's control with a +1/+1 counter on it" (CR 603.7c, 122.6).
fn return_with_counter(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Target::Object(card) = item.subject else { return Ok(()) };
    if !is_live(g, card) { return Ok(()) }
    let owner = g.obj(card).owner;
    g.put_onto_battlefield_with_counters(card, owner, false, &[(CounterKind::PlusOne, 1)])?;
    Ok(())
}

static MODAL: ModalDef = modes(&[
    mode(effects::return_first_target_to_hand, "Return target non-Doctor permanent card from your graveyard to your hand.")
        .targets(&[target(non_doctor_permanent_in_your_graveyard)]),
    mode(watch_for_death, "Choose up to one other target creature. When it dies this turn, return that card to the battlefield under its owner's control with a +1/+1 counter on it at the beginning of the next end step.")
        .targets(&[targets::OTHER_CREATURE.optional()]),
]);

pub static CARD: CardDef = CardDef {
    abilities: &[modal_triggered(triggers::THIS_ENTERS, &MODAL, "When Dr. McCoy enters, choose one — Return target non-Doctor permanent card from your graveyard to your hand; or choose up to one other target creature. When it dies this turn, return that card to the battlefield under its owner's control with a +1/+1 counter on it at the beginning of the next end step.")],
    delayed: &[
        triggered(triggers::THIS_DIES, return_at_end_step, "When it dies this turn, return that card at the beginning of the next end step."),
        delayed_trigger(return_with_counter, "Return that card to the battlefield under its owner's control with a +1/+1 counter on it."),
    ],
    ..oracle::DR_LEONARD_BONES_MCCOY
};
