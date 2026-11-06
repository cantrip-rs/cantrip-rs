use cantrip_api::*;
use cantrip_mechanics::effects;
use cantrip_oracle as oracle;

/// "Whenever this creature deals damage" (CR 603.2).
fn when_creature_deals_damage(
    q: &dyn Query,
    asker: &Asker,
    event: &Event,
) -> Option<Fired> {
    let Event::Damage { source, .. } = event else { return None };
    let source = *source;
    let subject = Target::Object(source);
    if !q.state().obj(source).zone.is(ZoneKind::Stack) && asker.source == source {
        return Fired::about(subject);
    }
    None
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
                when_asked(EventMask::DAMAGE, when_creature_deals_damage),
                effects::gain_matched_damage_life,
                "Whenever this creature deals damage, you gain that much life.",
            )
            .sums_per_subject()
            .damage_amount(),
    ],
    ..oracle::EL_HAJJ_J
};
