use cantrip_api::*;
use cantrip_mechanics::effects;
use cantrip_oracle as oracle;

/// "Whenever one or more +1/+1 counters are put on another non-Hydra creature you control" (CR 122.6, 603.2c): one trigger
/// per creature the counters went on, however many were put on it (the core announces only positive amounts, and also
/// when a creature enters with counters). "Non-Hydra" reads the Hydra word, which a text-changing effect can change (CR 612.2).
fn plus_one_counters_put_on_another_non_hydra(q: &dyn Query, asker: &Asker, event: &Event) -> Option<Fired> {
    let Event::CountersPut { object, kind: CounterKind::PlusOne, amount, .. } = event else { return None };
    let id = *object;
    let chars = q.chars(id);
    if id != asker.source
        && chars.is(CardType::Creature)
        && !chars.has_subtype(q.words(asker.source).subtype(Subtype::Hydra))
        && q.controller(id) == asker.controller
    {
        return Some(Fired { subject: Target::Object(id), data: [*amount as u32, id.0] });
    }
    None
}

pub static CARD: CardDef = CardDef {
    enters: Some(effects::enters_with_x_counters::<{ CounterKind::PlusOne as usize }>),
    abilities: &[triggered(
        when_asked(EventMask::COUNTERS, plus_one_counters_put_on_another_non_hydra),
        effects::plus_one_counter_on_this,
        "Whenever one or more +1/+1 counters are put on another non-Hydra creature you control, put a +1/+1 counter on this creature.",
    )
    .once_per_batch()
    .batch_by_data()],
    ..oracle::WILDWOOD_SCOURGE
};
