use cantrip_api::*;
use cantrip_mechanics::{replacements, statics};
use cantrip_oracle as oracle;

/// "Other creatures you control with counters on them have hexproof" (CR 613.1f, 702.11b): any counter kind, on
/// another creature of this source's controller, read afresh as layers apply.
fn other_creatures_with_counters(view: &dyn LayerQuery, source: ObjectId, affected: ObjectId, chars: &Chars) -> bool {
    affected != source && chars.is(CardType::Creature)
        && view.state().obj(affected).controller == view.state().obj(source).controller
        && view.state().obj(affected).has_counters()
}

/// "If you would put one or more counters on a creature you control" (CR 122.6, 122.6a, 614.1a): the putter and
/// creature's controller must both be you, including counters with which it enters. Costs also qualify.
fn you_put_counters_on_your_creature(q: &dyn Query, asker: &Asker, proposal: &Proposal) -> bool {
    let Proposal::Counters(c) = proposal else { return false };
    c.putter == asker.controller && c.controller == asker.controller
        && (c.entering || q.is_on_battlefield(c.object)) && q.chars(c.object).is(CardType::Creature)
        && c.counts.iter().any(|&n| n > 0)
}

/// "Put twice that many of each of those kinds of counters on that creature instead" (CR 614.5).
fn twice_that_many(_: &dyn Query, _: &Asker, proposal: &mut Proposal) {
    replacements::double_counters_apply(proposal);
}

pub static CARD: CardDef = CardDef {
    statics: &[static_ability(Layer::Ability, other_creatures_with_counters, statics::have_hexproof, "Other creatures you control with counters on them have hexproof.")],
    replacements: &[replacement(ReplacementKind::Counters, you_put_counters_on_your_creature, twice_that_many, "puts twice that many counters instead")],
    ..oracle::DR_JOSEPH_MBENGA
};
