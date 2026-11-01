use cantrip_api::*;
use cantrip_mechanics::triggers::{CREATURE_YOU_CONTROL_ENTERS, THIS_ENTERS, YOUR_END_STEP};
use cantrip_mechanics::{airbend_targets, exile_and_return, targets};
use cantrip_oracle as oracle;

/// "Whenever a creature you control enters, put a quest counter on this enchantment." The counter goes on the object
/// that triggered; if it has left the battlefield by now the core puts nothing on it.
fn quest_counter(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    g.add_counters(item.object, CounterKind::Quest, 1);
    Ok(())
}

/// The intervening "if this enchantment has four or more quest counters on it" (CR 603.4), checked as the end step begins
/// and again as the ability resolves.
fn four_quest_counters(q: &dyn Query, asker: &Asker) -> bool {
    q.state().obj(asker.source).counters(CounterKind::Quest) >= 4
}

/// "Exile up to one target creature you control, then return it to the battlefield under its owner's control": it comes
/// back as a new object (CR 400.7); a token exiled this way ceases to exist (CR 111.7). Nothing happens to a target that
/// is no longer legal (CR 608.2b).
fn flicker(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(creature) = targets::object(g, item, 0) else { return Ok(()) };
    exile_and_return(g, &[creature])
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(THIS_ENTERS, airbend_targets, "When this enchantment enters, airbend up to one target creature.")
            .targets(&[targets::CREATURE.between(0, 1)]),
        triggered(CREATURE_YOU_CONTROL_ENTERS, quest_counter, "Whenever a creature you control enters, put a quest counter on this enchantment."),
        triggered(
            YOUR_END_STEP,
            flicker,
            "At the beginning of your end step, if this enchantment has four or more quest counters on it, exile up to one target creature you control, then return it to the battlefield under its owner's control.",
        )
        .targets(&[targets::CREATURE_YOU_CONTROL.between(0, 1)])
        .condition(four_quest_counters),
    ],
    ..oracle::AIRBENDER_ASCENSION
};
