use cantrip_api::*;
use cantrip_mechanics::{
    attach::{attached_to_it, tap_enchanted},
    targets,
    triggers::THIS_ENTERS,
};
use cantrip_oracle as oracle;

/// "Enchanted permanent's activated abilities can't be activated" (CR 602.5, 605.1a): every activated ability of the
/// permanent this Aura is attached to - mana abilities too - whoever would activate it. The Aura's rule, not an ability of
/// the permanent (CR 113.1a), so the permanent losing its abilities leaves it in force (CR 613.1f).
fn enchanted_permanent_cant_activate(q: &dyn Query, asker: &Asker, source: ObjectId, _: &ActivatedDef) -> bool {
    !attached_to_it(q, asker, source)
}

pub static CARD: CardDef = CardDef {
    enchant: Some(targets::ARTIFACT_OR_CREATURE),
    abilities: &[triggered(THIS_ENTERS, tap_enchanted, "When this Aura enters, tap enchanted permanent.")],
    untap_bans: &[attached_to_it],
    activation_rules: &[enchanted_permanent_cant_activate],
    ..oracle::STUCK_IN_SUMMONERS_SANCTUM
};
