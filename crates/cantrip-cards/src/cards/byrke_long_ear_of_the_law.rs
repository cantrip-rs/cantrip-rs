use cantrip_api::*;
use cantrip_mechanics::{double, effects, targets, triggers};
use cantrip_oracle as oracle;

/// "Whenever a creature you control with a +1/+1 counter on it attacks" (CR 508.3a, 603.2, 122.1): a creature its controller has
/// attacks (the shared matcher, about that attacker) and, as the attack is declared, has a +1/+1 counter on it. It is not an
/// intervening "if", so the counters are read again as the ability resolves.
fn creature_you_control_with_a_plus_one_counter_attacks(q: &dyn Query, me: &Asker, event: &Event) -> Option<Fired> {
    let fired = triggers::creature_you_control_attacks(q, me, event)?;
    let Target::Object(attacker) = fired.subject else { return None };
    (q.obj(attacker).counters(CounterKind::PlusOne) > 0).then_some(fired)
}

/// "Double the number of +1/+1 counters on it" (CR 701.10e): the attacker, as the ability resolves.
fn double_the_attackers_plus_one_counters(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if let Target::Object(attacker) = item.subject {
        double::counters(g, &[attacker], CounterKind::PlusOne);
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            triggers::THIS_ENTERS,
            effects::plus_one_counter_on_each_target,
            "When this creature enters, put a +1/+1 counter on each of up to two target creatures.",
        )
        .targets(&[targets::CREATURE.between(0, 2)]),
        triggered(
            when_asked(EventMask::ATTACKS, creature_you_control_with_a_plus_one_counter_attacks),
            double_the_attackers_plus_one_counters,
            "Whenever a creature you control with a +1/+1 counter on it attacks, double the number of +1/+1 counters on it.",
        ),
    ],
    ..oracle::BYRKE_LONG_EAR_OF_THE_LAW
};
