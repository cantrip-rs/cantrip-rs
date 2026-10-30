use cantrip_api::*;
use cantrip_mechanics::effects::{destroy_first_target, sacrifice_to_destroy_artifact_or_enchantment};
use cantrip_mechanics::targets;
use cantrip_mechanics::triggers;
use cantrip_oracle as oracle;

/// "Target artifact or enchantment defending player controls" (CR 508.5): the player the source attacks, or the
/// controller of the planeswalker or the protector of the battle it attacks - still so once that planeswalker or battle
/// has been removed from combat (`Attacker::defending`).
fn defending_players_artifact_or_enchantment(q: &dyn Query, asker: &Asker, t: Target) -> bool {
    if !q.state().attackers.iter().any(|a| a.id == asker.source) {
        return false;
    }
    let Some(defending) = q.defending_player(asker.source) else { return false };
    (targets::ARTIFACT_OR_ENCHANTMENT.filter)(q, asker, t) && matches!(t, Target::Object(id) if q.controller(id) == defending)
}

pub static BACK: CardDef = CardDef {
    abilities: &[
        sacrifice_to_destroy_artifact_or_enchantment(1, "{1}, Sacrifice this creature: Destroy target artifact or enchantment."),
        triggered(triggers::THIS_ATTACKS, destroy_first_target, "Whenever this creature attacks, destroy target artifact or enchantment defending player controls.")
            .targets(&[target(defending_players_artifact_or_enchantment)]),
    ],
    ..oracle::FRENZIED_TRAPBREAKER
};

pub static CARD: CardDef = CardDef {
    abilities: &[sacrifice_to_destroy_artifact_or_enchantment(1, "{1}, Sacrifice this creature: Destroy target artifact or enchantment.")],
    back: Some(&BACK),
    ..oracle::OUTLAND_LIBERATOR
};
