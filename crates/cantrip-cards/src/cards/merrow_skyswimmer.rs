use cantrip_api::*;
use cantrip_mechanics::{tokens, triggers::THIS_ENTERS};
use cantrip_oracle as oracle;

/// "When this creature enters, create a 1/1 white and blue Merfolk creature token." Convoke, flying and vigilance are the
/// core's, read from the card's JSON.
fn make_merfolk(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    tokens::create(g, item.controller, &tokens::MERFOLK_WHITE_BLUE, 1)?;
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(THIS_ENTERS, make_merfolk, "When this creature enters, create a 1/1 white and blue Merfolk creature token.")],
    ..oracle::MERROW_SKYSWIMMER
};

pub static EXTRA: &[&CardDef] = &[&tokens::MERFOLK_WHITE_BLUE];
