use cantrip_api::*;
use cantrip_mechanics::{effects, lands, statics, triggers};
use cantrip_oracle as oracle;

/// "Each creature that's enchanted by an Aura you control can't attack you or planeswalkers you control" (CR 508.1c, 303.4): a
/// creature with an Aura attached that Eriette's controller controls (a Role token counts) may not attack that player or their
/// planeswalkers; an Aura an opponent controls on it, or an Aura attached to something else, does not bar it, and its attacks on
/// other players, planeswalkers and battles stay open. A creature already attacking stays so (CR 506.4a).
fn enchanted_by_your_aura_cant_attack_you(q: &dyn Query, asker: &Asker, creature: ObjectId, at: Target) -> bool {
    !statics::attacks_you_or_your_planeswalker(q, asker, at)
        || !q.state().battlefield().iter().any(|&id| {
            q.attached_to(id) == Some(creature) && q.controller(id) == asker.controller && q.chars(id).has_subtype(Subtype::Aura)
        })
}

pub static CARD: CardDef = CardDef {
    attack_rules: &[enchanted_by_your_aura_cant_attack_you],
    abilities: &[triggered(
        triggers::YOUR_END_STEP,
        effects::each_opponent_loses_life_you_gain_life_for_each_subtype_you_control::<{ lands::sub(Subtype::Aura) }>,
        "At the beginning of your end step, each opponent loses X life and you gain X life, where X is the number of Auras you control.",
    )],
    ..oracle::ERIETTE_OF_THE_CHARMED_APPLE
};
