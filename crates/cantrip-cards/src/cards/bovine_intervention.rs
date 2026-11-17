use cantrip_api::*;
use cantrip_mechanics::{effects, targets, tokens};
use cantrip_oracle as oracle;

/// "Destroy target artifact or creature. Its controller creates a 2/2 white Ox creature token."
fn effect(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    effects::destroy_and_its_controller_creates(g, item, &tokens::OX)
}

pub static EXTRA: &[&CardDef] = &[&tokens::OX];

pub static CARD: CardDef = CardDef { spell: spell(&[targets::ARTIFACT_OR_CREATURE], effect), ..oracle::BOVINE_INTERVENTION };
