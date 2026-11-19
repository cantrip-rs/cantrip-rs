use cantrip_api::*;
use cantrip_mechanics::{effects, explore, targets};
use cantrip_oracle as oracle;

/// Mode 2: "Target creature you control explores, then it explores again." (CR 701.44a) Each explore is whole before the next: a
/// nonland card the first reveals may be put into the graveyard or left on top, where the second reveals it again. With the target
/// gone as this resolves the spell does nothing (CR 608.2b).
fn explores_twice(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(creature) = targets::object(g, item, 0) else { return Ok(()) };
    explore(g, creature)?;
    explore(g, creature)
}

static MODES: ModalDef = modes(&[
    mode(effects::destroy_first_target, "Destroy target artifact or enchantment.").targets(&[targets::ARTIFACT_OR_ENCHANTMENT]),
    mode(explores_twice, "Target creature you control explores, then it explores again.").targets(&[targets::CREATURE_YOU_CONTROL]),
]);

pub static CARD: CardDef = CardDef { spell: Some(SpellDef::MODAL), modal: Some(&MODES), ..oracle::OVER_THE_EDGE };
