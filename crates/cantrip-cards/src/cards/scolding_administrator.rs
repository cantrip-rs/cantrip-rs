use cantrip_api::*;
use cantrip_mechanics::triggers::{REPARTEE, THIS_DIES};
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

/// "If it had counters on it" (CR 603.4, 603.10a): the creature as it last was on the battlefield, counters of any kind.
fn had_counters(q: &dyn Query, asker: &Asker) -> bool {
    CounterKind::ALL.iter().any(|&kind| q.obj(asker.source).counters(kind) != 0)
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            REPARTEE,
            effects::plus_one_counter_on_this,
            "Repartee \u{2014} Whenever you cast an instant or sorcery spell that targets a creature, put a +1/+1 counter on this creature.",
        ),
        // "If it had counters on it" (CR 603.4, 603.10a): looked back at as it last existed, checked as it dies and again as the
        // ability resolves. The counters it had are all put on, every kind (CR 122.6).
        triggered(
            THIS_DIES,
            effects::put_its_counters_on_target,
            "When this creature dies, if it had counters on it, put those counters on up to one target creature.",
        )
        .condition(had_counters)
        .targets(&[targets::CREATURE.optional()]),
    ],
    ..oracle::SCOLDING_ADMINISTRATOR
};
