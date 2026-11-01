use cantrip_api::*;
use cantrip_mechanics::{airbend_targets, destroy_target, targets};
use cantrip_oracle as oracle;

/// "Destroy target attacking creature." (CR 701.8)
fn destroy_attacking_creature(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    destroy_target(g, item, 0);
    Ok(())
}

static MODAL: ModalDef = modes(&[
    mode(destroy_attacking_creature, "Destroy target attacking creature.")
        .targets(&[targets::ATTACKING_CREATURE]),
    mode(airbend_targets, "Airbend target creature you control.")
        .targets(&[targets::CREATURE_YOU_CONTROL]),
]);

pub static CARD: CardDef = CardDef { spell: Some(SpellDef::MODAL), modal: Some(&MODAL), ..oracle::AIRBENDERS_REVERSAL };
