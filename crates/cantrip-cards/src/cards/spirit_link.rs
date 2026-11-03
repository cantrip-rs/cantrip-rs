use cantrip_api::*;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

/// "Whenever enchanted creature deals damage" (CR 603.2).
fn when_enchanted_creature_deals_damage(
    q: &dyn Query,
    asker: &Asker,
    event: &Event,
) -> Option<Fired> {
    let Event::Damage { source, .. } = event else { return None };
    let source = *source;
    let subject = Target::Object(source);
    let data = [0, source.0];
    if !q.state().obj(source).zone.is(ZoneKind::Stack)
        && Some(asker.source).and_then(|o: ObjectId| q.state().obj(o).attached_to)
            == Some(source)
    {
        return Some(Fired { subject, data });
    }
    None
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
                when_asked(EventMask::DAMAGE, when_enchanted_creature_deals_damage),
                effects::gain_matched_damage_life,
                "Whenever enchanted creature deals damage, you gain that much life.",
            ).damage_amount()
            .sums_per_subject(),
    ],
    enchant: Some(targets::CREATURE),
    ..oracle::SPIRIT_LINK
};
