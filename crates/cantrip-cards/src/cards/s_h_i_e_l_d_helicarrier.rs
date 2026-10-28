use cantrip_api::*;
use cantrip_mechanics::crew::crew;
use cantrip_mechanics::{tokens, triggers};
use cantrip_oracle as oracle;

/// "Create two 1/1 white Soldier creature tokens", together (CR 101.4).
fn make_soldiers(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    tokens::create(g, item.controller, &tokens::SOLDIER, 2)?;
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(triggers::THIS_ENTERS, make_soldiers, "When this Vehicle enters, create two 1/1 white Soldier creature tokens."),
        crew(6, "Crew 6 (Tap any number of creatures you control with total power 6 or more: This Vehicle becomes an artifact creature until end of turn.)"),
    ],
    ..oracle::S_H_I_E_L_D_HELICARRIER
};

pub static EXTRA: &[&CardDef] = &[&tokens::SOLDIER];
