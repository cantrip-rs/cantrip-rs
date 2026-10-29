use cantrip_api::*;
use cantrip_mechanics::{targets, triggers};
use cantrip_oracle as oracle;

/// "When Dr. McCoy enters, put a +1/+1 counter on another target creature you control."
fn counter_another(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(creature) = targets::object(g, item, 0) else { return Ok(()) };
    g.add_counters(creature, CounterKind::PlusOne, 1);
    Ok(())
}

/// The destruction is replaced only while the creature is still being destroyed into its owner's graveyard, and only if
/// it is another creature controlled by this source's controller with a +1/+1 counter. CR 614.6: if an earlier effect
/// changes the destination, this effect no longer applies. `then` removes counters and heals after the move is replaced.
fn another_countered_creature_would_be_destroyed(q: &dyn Query, asker: &Asker, proposal: &Proposal) -> bool {
    let Proposal::ZoneChange(z) = proposal else { return false };
    z.destruction()
        && z.from.is(ZoneKind::Battlefield)
        && z.to == Zone::graveyard(q.obj(z.object).owner)
        && z.object != asker.source
        && q.chars(z.object).is(CardType::Creature)
        && q.obj(z.object).controller == asker.controller
        && q.state().obj(z.object).counters(CounterKind::PlusOne) > 0
}

fn replace_destruction(_: &dyn Query, _: &Asker, proposal: &mut Proposal) {
    if let Proposal::ZoneChange(z) = proposal {
        z.replaced = true;
    }
}

fn remove_counters_and_heal(g: &mut dyn Game, _: &Asker, before: &Proposal, _: &Proposal) {
    if let Proposal::ZoneChange(z) = before {
        let amount = g.obj(z.object).counters(CounterKind::PlusOne);
        g.remove_counters(z.object, CounterKind::PlusOne, amount);
        g.heal(z.object);
    }
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(triggers::THIS_ENTERS, counter_another, "When Dr. McCoy enters, put a +1/+1 counter on another target creature you control.")
        .targets(&[targets::OTHER_CREATURE_YOU_CONTROL])],
    replacements: &[replacement(
        ReplacementKind::ZoneChange,
        another_countered_creature_would_be_destroyed,
        replace_destruction,
        "removes all +1/+1 counters and heals it instead of destroying it",
    )
    .then(remove_counters_and_heal)],
    ..oracle::DR_MCCOY_SURLY_SURGEON
};
