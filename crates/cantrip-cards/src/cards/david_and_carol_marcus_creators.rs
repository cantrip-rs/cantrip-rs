use cantrip_api::*;
use cantrip_mechanics::{tokens, triggers};
use cantrip_oracle as oracle;

/// "Create a tapped Planet land token named New Planet with '{T}: Add one mana of any color.'" (CR 111.1, 111.4, 603.6a).
fn create_new_planet(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    tokens::create_by_rule(g, item.controller, &tokens::NEW_PLANET, 1, true)?;
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(triggers::THIS_ENTERS, create_new_planet, "When David and Carol Marcus enter, create a tapped Planet land token named New Planet with \"{T}: Add one mana of any color.\"")],
    ..oracle::DAVID_AND_CAROL_MARCUS_CREATORS
};

pub static EXTRA: &[&CardDef] = &[&tokens::NEW_PLANET];
