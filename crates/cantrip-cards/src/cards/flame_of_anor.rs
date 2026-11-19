use cantrip_api::*;
use cantrip_mechanics::{effects, lands, statics, targets};
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

/// "If you control a Wizard as you cast this spell, you may choose two instead" (CR 700.2): checked as modes are announced,
/// so a Wizard that leaves in response leaves both modes chosen.
static MODAL: ModalDef = modes(&[
    mode(draw_two, "Target player draws two cards.")
        .targets(&[targets::PLAYER]),
    mode(destroy_artifact, "Destroy target artifact.")
        .targets(&[targets::ARTIFACT]),
    mode(effects::damage_first_target::<5>, "Flame of Anor deals 5 damage to target creature.")
        .targets(&[targets::CREATURE]),
])
    .up_to_if(statics::you_control_a::<{ lands::sub(Subtype::Wizard) }>, 2);

pub static CARD: CardDef = CardDef { spell: Some(SpellDef::MODAL), modal: Some(&MODAL), ..oracle::FLAME_OF_ANOR };
