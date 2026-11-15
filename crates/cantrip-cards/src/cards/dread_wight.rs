use cantrip_api::*;
use cantrip_mechanics::{effects, triggers};
use cantrip_oracle as oracle;

/// "At end of combat, put a paralyzation counter on each creature blocking or blocked by this creature and tap those creatures. Each of those creatures doesn't untap during its controller's untap step for as long as it has a paralyzation counter on it. Each of those creatures gains "{4}: Remove a paralyzation counter from this creature."" (CR 608.2h, 611.2a, 122.1).
fn paralyze_blockers_and_blocked(
    g: &mut dyn Game,
    item: &StackItem,
) -> Result<(), Suspend> {
    // Last-known information if this creature has left combat (CR 608.2h).
    let q: &dyn Query = g;
    let mut creatures = q.creatures_blocking(item.object);
    creatures.extend(q.creatures_blocked_by(item.object));
    let counters: Vec<(ObjectId, CounterKind, i32)> = creatures.iter().map(|&c| (c, CounterKind::Paralyzation, 1)).collect();
    g.add_counters_together(&counters);
    g.tap_together(&creatures);
    g.end_sentence();
    instruction(g, |g| {
        for &creature in &creatures {
            g.add_effect(item.object, creature, EffectKind::DoesntUntap, Duration::WhileCounter(CounterKind::Paralyzation));
        }
    });
    g.end_sentence();
    let granted = EffectKind::AddAbility(AbilityRef::delayed(item.ability.card, 0));
    instruction(g, |g| {
        for &creature in &creatures {
            g.add_effect(item.object, creature, granted, Duration::Indefinite);
        }
    });
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            triggers::END_OF_COMBAT,
            paralyze_blockers_and_blocked,
            "At end of combat, put a paralyzation counter on each creature blocking or blocked by this creature and tap those creatures. Each of those creatures doesn't untap during its controller's untap step for as long as it has a paralyzation counter on it. Each of those creatures gains \"{4}: Remove a paralyzation counter from this creature.\"",
        ),
    ],
    delayed: &[
        ability(
            cost("{4}"),
            effects::remove_a_counter_from_this::<{ CounterKind::Paralyzation as usize }>,
            "{4}: Remove a paralyzation counter from this creature.",
        ),
    ],
    ..oracle::DREAD_WIGHT
};
