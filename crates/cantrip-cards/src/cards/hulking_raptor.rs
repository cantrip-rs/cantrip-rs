use cantrip_api::*;
use cantrip_mechanics::triggers::{ward, YOUR_FIRST_MAIN_PHASE};
use cantrip_oracle as oracle;

/// "At the beginning of your first main phase, add {G}{G}" (CR 106.4, 505.1a): the mana empties at the end of that phase.
fn add_green_green(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    g.add_mana(item.controller, Mana::G, 2);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        ward::<2>("Ward {2} (Whenever this creature becomes the target of a spell or ability an opponent controls, counter it unless that player pays {2}.)"),
        triggered(YOUR_FIRST_MAIN_PHASE, add_green_green, "At the beginning of your first main phase, add {G}{G}.").produces_mana(),
    ],
    ..oracle::HULKING_RAPTOR
};
