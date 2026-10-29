use cantrip_api::*;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

/// "Destroy target artifact or creature. You gain 1 life." The life is gained whether or not the target was destroyed
/// (indestructible, regenerated); with its only target illegal the spell does nothing at all (CR 608.2b).
fn destroy_and_gain_a_life(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    effects::destroy_first_target(g, item)?;
    effects::gain_one_life(g, item)
}

pub static CARD: CardDef = CardDef { spell: spell(&[targets::ARTIFACT_OR_CREATURE], destroy_and_gain_a_life), ..oracle::GRAPPLE_WITH_DEATH };
