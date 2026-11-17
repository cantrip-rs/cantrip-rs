use cantrip_api::*;
use cantrip_mechanics::tokens;
use cantrip_oracle as oracle;

/// "Create four 1/1 white Dog creature tokens."
fn effect(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    tokens::create(g, item.controller, &tokens::DOG, 4)?;
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[], effect),
    ..oracle::RELEASE_THE_DOGS
};

pub static EXTRA: &[&CardDef] = &[&tokens::DOG];
