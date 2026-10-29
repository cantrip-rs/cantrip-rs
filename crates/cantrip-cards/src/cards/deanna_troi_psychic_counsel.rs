use cantrip_api::*;
use cantrip_oracle as oracle;

/// "Other creatures you control enter with an additional +1/+1 counter on them." (CR 614.1c, 614.12, 122.6): only a
/// creature entering under this ability's controller, and never Deanna herself, gets the extra counter.
fn other_creatures_enter_with_a_counter(q: &dyn Query, asker: &Asker, proposal: &Proposal) -> bool {
    matches!(proposal, Proposal::ZoneChange(z)
        if z.to.is(ZoneKind::Battlefield)
            && z.controller == asker.controller
            && z.object != asker.source
            && q.chars(z.object).is(CardType::Creature))
}

fn add_a_counter(_: &dyn Query, _: &Asker, proposal: &mut Proposal) {
    if let Proposal::ZoneChange(z) = proposal {
        z.counters[CounterKind::PlusOne as usize] += 1;
    }
}

pub static CARD: CardDef = CardDef {
    replacements: &[replacement(
        ReplacementKind::ZoneChange,
        other_creatures_enter_with_a_counter,
        add_a_counter,
        "enters with an additional +1/+1 counter",
    )],
    ..oracle::DEANNA_TROI_PSYCHIC_COUNSEL
};
