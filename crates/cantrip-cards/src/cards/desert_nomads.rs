use cantrip_api::*;
use cantrip_oracle as oracle;

/// "Prevent all damage that would be dealt to this creature by Deserts." (CR 615.1).
fn prevent_all_damage_would_be_applies(
    q: &dyn Query,
    asker: &Asker,
    e: &DamageEvent,
) -> bool {
    {
        let (source, target) = (e.source, &e.target);
        if !q.state().obj(source).zone.is(ZoneKind::Stack)
            && q
                .chars(source)
                .has_subtype(q.words(asker.source).subtype(Subtype::Desert))
            && matches!(* target, Target::Object(o) if asker.source == o)
        {
            return true;
        }
    }
    false
}

pub static CARD: CardDef = CardDef {
    preventions: &[
        PreventionDef {
            hint: "prevents all damage that would be dealt to this creature by Deserts",
            applies: prevent_all_damage_would_be_applies,
            amount: PreventionAmount::All,
        },
    ],
    ..oracle::DESERT_NOMADS
};
