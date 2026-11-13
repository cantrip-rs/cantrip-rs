use cantrip_api::*;
use cantrip_mechanics::crew::crew;
use cantrip_mechanics::targets;
use cantrip_oracle as oracle;

/// "Each other Vehicle and creature you control enters ..." (CR 614.1c, 614.12): another permanent about to enter the battlefield
/// under this permanent's controller that is a Vehicle or a creature as it would exist there, card or token alike. The
/// Velocipede itself is not "other"; nor is it on the battlefield as it enters, so this ability does not apply to it, nor to
/// a Vehicle or creature entering at the same time as it.
fn another_vehicle_or_creature_enters_under_you(q: &dyn Query, asker: &Asker, proposal: &Proposal) -> bool {
    matches!(proposal, Proposal::ZoneChange(z)
        if z.to.is(ZoneKind::Battlefield)
            && z.object != asker.source
            && z.controller == asker.controller
            && targets::is_creature_or_vehicle(&q.chars(z.object)))
}

/// "... with an additional +1/+1 counter on it if its mana value is 4 or less. Otherwise, it enters with three additional
/// +1/+1 counters on it." The mana value is the one the object has as it enters (`Query::mana_value`: a token's copy keeps its
/// copiable mana value, an {X} in a cost is 0, CR 202.3e).
fn add_plus_one_counters_by_mana_value(q: &dyn Query, _: &Asker, proposal: &mut Proposal) {
    let Proposal::ZoneChange(z) = proposal else { return };
    let counters = if q.mana_value(z.object) <= 4 { 1 } else { 3 };
    z.counters[CounterKind::PlusOne as usize] += counters;
}

pub static CARD: CardDef = CardDef {
    abilities: &[crew(3, "Crew 3")],
    replacements: &[replacement(
        ReplacementKind::ZoneChange,
        another_vehicle_or_creature_enters_under_you,
        add_plus_one_counters_by_mana_value,
        "Each other Vehicle and creature you control enters with an additional +1/+1 counter on it if its mana value is 4 or less. Otherwise, it enters with three additional +1/+1 counters on it.",
    )],
    ..oracle::THUNDEROUS_VELOCIPEDE
};
