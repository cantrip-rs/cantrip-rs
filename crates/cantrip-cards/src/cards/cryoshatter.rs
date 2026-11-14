use cantrip_api::*;
use cantrip_mechanics::{attach::attached_object, effects, statics::plus_pt, targets};
use cantrip_oracle as oracle;

/// "When enchanted creature becomes tapped or is dealt damage" (CR 603.2, 701.26a, 120.2): the creature this Aura enchants
/// is tapped by any path, or is dealt damage (an event the engine records only for damage above zero, CR 120.2). The creature is the trigger's subject:
/// "destroy it" destroys that creature even if Cryoshatter has left the battlefield or enchants another creature by then.
fn enchanted_creature_becomes_tapped_or_is_dealt_damage(q: &dyn Query, source: ObjectId, event: &Event) -> Option<Fired> {
    match event {
        Event::Tapped { permanent } if q.attached_to(source) == Some(*permanent) => Fired::about(Target::Object(*permanent)),
        Event::Damage { target: Target::Object(hurt), .. } if q.attached_to(source) == Some(*hurt) => Fired::about(Target::Object(*hurt)),
        _ => None,
    }
}

pub static CARD: CardDef = CardDef {
    enchant: Some(targets::CREATURE),
    abilities: &[triggered(
        when(EventMask(EventMask::TAPPED.0 | EventMask::DAMAGE.0), enchanted_creature_becomes_tapped_or_is_dealt_damage),
        effects::destroy_subject,
        "When enchanted creature becomes tapped or is dealt damage, destroy it.",
    )
    // Damage dealt to it by several sources at once is one batch (CR 603.2c): the one trigger.
    .sums_per_subject()],
    statics: &[static_ability(Layer::PtModify, attached_object, plus_pt::<-5, 0>, "Enchanted creature gets -5/-0.")],
    ..oracle::CRYOSHATTER
};
