use cantrip_api::*;
use cantrip_mechanics::{destroy_target_no_regen, targets, triggers};
use cantrip_oracle as oracle;

/// "Target nonartifact creature that player controls of their choice" (CR 115.1, 603.3d).
fn upkeep_players_nonartifact_creature(q: &dyn Query, asker: &Asker, target: Target) -> bool {
    (targets::NONARTIFACT_CREATURE.filter)(q, asker, target)
        && target.object().is_some_and(|id| q.event_subject().player() == Some(q.controller(id)))
}

/// "Destroy target nonartifact creature that player controls of their choice. It can't be regenerated." (CR 701.8, 701.19c).
fn destroy(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    destroy_target_no_regen(g, item, 0);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(triggers::EACH_PLAYERS_UPKEEP, destroy,
        "At the beginning of each player's upkeep, destroy target nonartifact creature that player controls of their choice. It can't be regenerated.")
        .targets(&[target(upkeep_players_nonartifact_creature)])
        .subject_chooses_targets()],
    ..oracle::THE_ABYSS
};
