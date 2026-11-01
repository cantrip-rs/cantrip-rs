use cantrip_api::*;
use cantrip_mechanics::{tokens, triggers};
use cantrip_oracle as oracle;

/// "Create a 1/1 white Ally creature token." (CR 111.1.)
fn ally(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    tokens::create(g, item.controller, &tokens::ALLY, 1)?;
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(triggers::THIS_ENTERS, ally, "When this creature enters, create a 1/1 white Ally creature token.")],
    ..oracle::INVASION_REINFORCEMENTS
};

pub static EXTRA: &[&CardDef] = &[&tokens::ALLY];
