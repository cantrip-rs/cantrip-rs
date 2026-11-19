use cantrip_api::*;
use cantrip_mechanics::{destroy_target, effects, targets};
use cantrip_oracle as oracle;

/// Destroy target artifact.
fn destroy_artifact(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    destroy_target(g, item, 0);
    Ok(())
}

static MODAL: ModalDef = modes(&[
    mode(destroy_artifact, "Destroy target artifact.")
        .targets(&[targets::ARTIFACT]),
    mode(effects::change_first_target, "Change the target of target spell or ability with a single target.")
        .targets(&[targets::SPELL_OR_ABILITY_WITH_A_SINGLE_TARGET]),
    mode(effects::each_target_cant_block_this_turn, "One or two target creatures can't block this turn.")
        .targets(&[targets::CREATURE.between(1, 2)]),
]);

pub static CARD: CardDef = CardDef { spell: Some(SpellDef::MODAL), modal: Some(&MODAL), ..oracle::UNTIMELY_MALFUNCTION };
