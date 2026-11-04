use cantrip_api::*;
use cantrip_mechanics::{
    attach::{attached_to_it, tap_attached},
    effects, targets,
    triggers::THIS_ENTERS,
};
use cantrip_oracle as oracle;

/// "When this Aura enters, tap enchanted creature." The creature it is attached to as the trigger resolves (CR 303.4m,
/// 608.2h).
fn tap_enchanted(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    tap_attached(g, item);
    Ok(())
}

/// "When enchanted creature is dealt damage" (CR 120.2, 603.2c): the creature this Aura is attached to as the damage is
/// dealt, whoever deals it and even when it is lethal (the engine records no damage event for damage of zero). The ability
/// is `.sums_per_subject()`, so the damage several sources deal it at once is one trigger.
fn enchanted_creature_dealt_damage(q: &dyn Query, source: ObjectId, event: &Event) -> Option<Fired> {
    match event {
        Event::Damage { target: Target::Object(hurt), .. } if q.attached_to(source) == Some(*hurt) => Fired::PLAIN,
        _ => None,
    }
}

pub static CARD: CardDef = CardDef {
    enchant: Some(targets::CREATURE),
    abilities: &[
        triggered(THIS_ENTERS, tap_enchanted, "When this Aura enters, tap enchanted creature."),
        triggered(when(EventMask::DAMAGE, enchanted_creature_dealt_damage), effects::sacrifice_self, "When enchanted creature is dealt damage, sacrifice this Aura.")
            .sums_per_subject(),
    ],
    untap_bans: &[attached_to_it],
    ..oracle::SLEEP_MAGIC
};
