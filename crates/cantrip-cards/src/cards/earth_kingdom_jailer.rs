use cantrip_api::*;
use cantrip_mechanics::{exile_target_until_leaves_effect, targets, triggers::THIS_ENTERS};
use cantrip_oracle as oracle;

/// "Up to one target artifact, creature, or enchantment an opponent controls with mana value 3 or greater" (CR 115.1, 202.3): on
/// the battlefield, controlled by an opponent, and judged again as the ability resolves (CR 608.2b).
fn artifact_creature_or_enchantment_an_opponent_controls_with_mana_value_3_or_greater(q: &dyn Query, asker: &Asker, t: Target) -> bool {
    targets::artifact_creature_or_enchantment_filter(q, asker, t)
        && matches!(t, Target::Object(id) if q.controller(id) != asker.controller && q.mana_value(id) >= 3)
}

/// "Exile up to one target ... until this creature leaves the battlefield" (CR 610.3): nothing is exiled if the Jailer already
/// left (CR 610.3b), and the exiled card returns as a new object when it leaves (CR 400.7).
pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        THIS_ENTERS,
        exile_target_until_leaves_effect,
        "When this creature enters, exile up to one target artifact, creature, or enchantment an opponent controls with mana value 3 or greater until this creature leaves the battlefield.",
    )
    .targets(&[up_to_one(artifact_creature_or_enchantment_an_opponent_controls_with_mana_value_3_or_greater)])],
    ..oracle::EARTH_KINGDOM_JAILER
};
