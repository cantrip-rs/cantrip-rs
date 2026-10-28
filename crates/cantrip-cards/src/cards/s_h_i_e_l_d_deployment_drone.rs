use cantrip_api::*;
use cantrip_mechanics::tokens::{self, SOLDIER};
use cantrip_mechanics::triggers::THIS_ENTERS;
use cantrip_oracle as oracle;

/// "Create a 1/1 white Soldier creature token."
fn soldier(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    tokens::create(g, item.controller, &SOLDIER, 1)?;
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(THIS_ENTERS, soldier, "When this creature enters, create a 1/1 white Soldier creature token.")],
    ..oracle::S_H_I_E_L_D_DEPLOYMENT_DRONE
};

pub static EXTRA: &[&CardDef] = &[&SOLDIER];
