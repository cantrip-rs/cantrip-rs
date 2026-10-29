use cantrip_api::*;
use cantrip_mechanics::{persist, targets, triggers::THIS_ENTERS};
use cantrip_oracle as oracle;

/// "Another target creature you control gains persist until end of turn" (CR 611.2a, 613.1f, 702.79a): a layer 6 effect that
/// gives it the card's delayed ability 0, a persist trigger of its own, until the cleanup step.
fn gain_persist(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(creature) = targets::object(g, item, 0) else { return Ok(()) };
    let this = g.card_id(&CARD);
    g.add_effect(item.object, creature, EffectKind::AddAbility(AbilityRef::delayed(this, 0)), Duration::EndOfTurn);
    Ok(())
}

/// "Remove any number of counters from target creature you control" (CR 122.1, 608.2d): the controller chooses which, of any
/// kinds, from none up to all of them. The most the request can name is 255 counters at once, far beyond a creature in a timed game.
fn remove_any_number_of_counters(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(creature) = targets::object(g, item, 0) else { return Ok(()) };
    g.remove_counters_up_to(item.controller, item.object, creature, u8::MAX)?;
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            THIS_ENTERS,
            gain_persist,
            "When Rhys enters, another target creature you control gains persist until end of turn.",
        )
        .targets(&[targets::OTHER_CREATURE_YOU_CONTROL]),
        ability(
            cost("{W}").and(TAP),
            remove_any_number_of_counters,
            "{W}, {T}: Remove any number of counters from target creature you control. Activate only as a sorcery.",
        )
        .targets(&[targets::CREATURE_YOU_CONTROL])
        .sorcery_speed(),
    ],
    // The granted ability (the card's delayed ability 0).
    delayed: &[persist::trigger("Persist (When this creature dies, if it had no -1/-1 counters on it, return it to the battlefield under its owner's control with a -1/-1 counter on it.)")],
    ..oracle::RHYS_THE_EVERMORE
};
