use cantrip_api::*;
use cantrip_mechanics::{destroy_target_no_regen, targets};
use cantrip_oracle as oracle;

/// Target artifact with mana value X (CR 115.1, 107.3a).
fn artifact_with_mana_value_x(q: &dyn Query, asker: &Asker, t: Target) -> bool {
    matches!(t, Target::Object(id) if (targets::ARTIFACT.filter)(q, asker, t) && targets::mana_value_is_x(q, id))
}

/// Destroy the artifact without regeneration, then damage its controller, using last known information if it left (CR 608.2h).
fn effect(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if let Some(artifact) = destroy_target_no_regen(g, item, 0) {
        g.end_sentence();
        g.damage(item.object, Target::Player(g.controller(artifact)), i64::from(item.x));
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[target(artifact_with_mana_value_x)], effect),
    late_targets: true,
    ..oracle::DETONATE
};
