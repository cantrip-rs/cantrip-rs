use cantrip_api::*;
use cantrip_mechanics::{tokens, triggers::LANDFALL};
use cantrip_oracle as oracle;

/// "Create a 4/4 green Beast creature token." (CR 111.1.)
fn create_beast(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    tokens::create(g, item.controller, &tokens::BEAST_4_4, 1)?;
    Ok(())
}

pub static EXTRA: &[&CardDef] = &[&tokens::BEAST_4_4];

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        LANDFALL,
        create_beast,
        "Landfall \u{2014} Whenever a land you control enters, create a 4/4 green Beast creature token.",
    )],
    ..oracle::RAMPAGING_BALOTHS
};
