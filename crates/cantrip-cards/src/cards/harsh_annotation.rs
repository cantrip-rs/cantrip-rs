use cantrip_api::*;
use cantrip_mechanics::{effects, targets, tokens};
use cantrip_oracle as oracle;

/// "Destroy target creature. Its controller creates a 1/1 white and black Inkling creature token with flying." The
/// controller is read before it is destroyed, and the token is made whether or not it was (indestructible, regenerated).
fn effect(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    effects::destroy_and_its_controller_creates(g, item, &tokens::INKLING)
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[targets::CREATURE], effect),
    ..oracle::HARSH_ANNOTATION
};

pub static EXTRA: &[&CardDef] = &[&tokens::INKLING];
