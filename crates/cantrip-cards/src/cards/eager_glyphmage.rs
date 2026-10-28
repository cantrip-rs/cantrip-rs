use cantrip_api::*;
use cantrip_mechanics::{tokens, triggers::THIS_ENTERS};
use cantrip_oracle as oracle;

/// "Create a 1/1 white and black Inkling creature token with flying."
fn inkling(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    tokens::create(g, item.controller, &tokens::INKLING, 1)?;
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(THIS_ENTERS, inkling, "When this creature enters, create a 1/1 white and black Inkling creature token with flying.")],
    ..oracle::EAGER_GLYPHMAGE
};

pub static EXTRA: &[&CardDef] = &[&tokens::INKLING];
