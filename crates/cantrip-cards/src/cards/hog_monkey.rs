use cantrip_api::*;
use cantrip_mechanics::{effects, targets, triggers::BEGINNING_OF_YOUR_COMBAT};
use cantrip_oracle as oracle;

/// "Target creature you control with a +1/+1 counter on it" (CR 115.1, 122.1): judged when the ability is put on the stack
/// (the ruling: it cannot be given the counter in response) and again as it resolves (CR 608.2b).
fn creature_you_control_with_a_plus_one_counter(q: &dyn Query, asker: &Asker, t: Target) -> bool {
    (targets::CREATURE_YOU_CONTROL.filter)(q, asker, t)
        && matches!(t, Target::Object(o) if q.obj(o).counters(CounterKind::PlusOne) > 0)
}

/// "... gains menace until end of turn" (CR 702.111, 611.2a).
fn target_gains_menace(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    effects::target_creature_gains_until_end_of_turn(g, item, Keyword::Menace)
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            BEGINNING_OF_YOUR_COMBAT,
            target_gains_menace,
            "At the beginning of combat on your turn, target creature you control with a +1/+1 counter on it gains menace until end of turn.",
        )
        .targets(&[target(creature_you_control_with_a_plus_one_counter)]),
        ability(
            cost("{5}"),
            effects::two_plus_one_counters_on_this,
            "Exhaust \u{2014} {5}: Put two +1/+1 counters on this creature. (Activate each exhaust ability only once.)",
        )
        .exhaust(),
    ],
    ..oracle::HOG_MONKEY
};
