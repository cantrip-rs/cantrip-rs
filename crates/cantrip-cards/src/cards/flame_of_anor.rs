use cantrip_api::*;
use cantrip_mechanics::{damage_target, effects, targets};
use cantrip_oracle as oracle;

/// Mode 0: target player draws two cards.
fn draw_two(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    effects::target_player_draws(g, item, 0, 2);
    Ok(())
}

/// Mode 1: destroy target artifact.
fn destroy_artifact(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    cantrip_mechanics::destroy_target(g, item, 0);
    Ok(())
}

/// Mode 2: Flame of Anor deals 5 damage to target creature.
fn damage(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    damage_target(g, item, 0, 5);
    Ok(())
}

/// "If you control a Wizard as you cast this spell, you may choose two instead" (CR 700.2): checked as modes are announced,
/// so a Wizard that leaves in response leaves both modes chosen.
fn has_a_wizard(q: &dyn Query, asker: &Asker) -> bool {
    let wizard = q.words(asker.source).subtype(Subtype::Wizard);
    q.permanents_of(asker.controller).into_iter().any(|id| q.chars(id).has_subtype(wizard))
}

static MODAL: ModalDef = modes(&[
    mode(draw_two, "Target player draws two cards.")
        .targets(&[targets::PLAYER]),
    mode(destroy_artifact, "Destroy target artifact.")
        .targets(&[targets::ARTIFACT]),
    mode(damage, "Flame of Anor deals 5 damage to target creature.")
        .targets(&[targets::CREATURE]),
])
    .up_to_if(has_a_wizard, 2);

pub static CARD: CardDef = CardDef { spell: Some(SpellDef::MODAL), modal: Some(&MODAL), ..oracle::FLAME_OF_ANOR };
