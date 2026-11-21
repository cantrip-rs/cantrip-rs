use cantrip_api::*;
use cantrip_mechanics::{raid, tokens, triggers::THIS_ATTACKS};
use cantrip_oracle as oracle;

/// "Create a 2/2 red Soldier creature token with firebending 1" (CR 111.1, 701.7).
fn create_a_firebending_soldier(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    tokens::create(g, item.controller, &tokens::FIREBENDING_SOLDIER, 1)?;
    Ok(())
}

pub static CARD: CardDef = CardDef {
    enters: Some(raid::enters_with_a_plus_one_counter),
    abilities: &[triggered(THIS_ATTACKS, create_a_firebending_soldier, "Whenever this creature attacks, create a 2/2 red Soldier creature token with firebending 1.")],
    ..oracle::CRUEL_ADMINISTRATOR
};

pub static EXTRA: &[&CardDef] = &[&tokens::FIREBENDING_SOLDIER];
