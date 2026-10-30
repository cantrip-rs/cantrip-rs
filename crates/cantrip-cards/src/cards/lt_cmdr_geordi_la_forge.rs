use cantrip_api::*;
use cantrip_oracle as oracle;

/// "If you would put one or more charge counters on an artifact you control" (CR 122.6, 614.1a): the putter and the
/// artifact's controller must both be you, the artifact is on the battlefield or entering with them, and the counters
/// put include a charge counter. Costs also qualify.
fn you_put_charge_counters_on_your_artifact(q: &dyn Query, asker: &Asker, proposal: &Proposal) -> bool {
    let Proposal::Counters(c) = proposal else { return false };
    c.putter == asker.controller && c.controller == asker.controller
        && (c.entering || q.is_on_battlefield(c.object)) && q.chars(c.object).is(CardType::Artifact)
        && c.counts[CounterKind::Charge as usize] > 0
}

/// "... put twice that many charge counters on that artifact instead" (CR 614.5): only the charge counters are doubled.
fn twice_that_many_charge_counters(_: &dyn Query, _: &Asker, proposal: &mut Proposal) {
    if let Proposal::Counters(c) = proposal {
        c.counts[CounterKind::Charge as usize] *= 2;
    }
}

pub static CARD: CardDef = CardDef {
    replacements: &[replacement(
        ReplacementKind::Counters,
        you_put_charge_counters_on_your_artifact,
        twice_that_many_charge_counters,
        "puts twice that many charge counters instead",
    )],
    ..oracle::LT_CMDR_GEORDI_LA_FORGE
};
