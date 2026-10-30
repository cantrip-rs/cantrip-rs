use cantrip_api::*;
use cantrip_mechanics::{tokens, triggers::THIS_ENTERS};
use cantrip_oracle as oracle;

/// The 1/1 white and blue Merfolk creature token Merrow Skyswimmer makes (CR 111.1, 205.4a).
pub static MERFOLK: CardDef = CardDef {
    colors: Some(ColorSet((1 << Color::White as u8) | (1 << Color::Blue as u8))),
    types: TypeSet::of(&[CardType::Creature]),
    power: Some(1),
    toughness: Some(1),
    ..token!(Merfolk)
};

/// "When this creature enters, create a 1/1 white and blue Merfolk creature token." Convoke, flying and vigilance are the
/// core's, read from the card's JSON.
fn make_merfolk(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    tokens::create(g, item.controller, &MERFOLK, 1)?;
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(THIS_ENTERS, make_merfolk, "When this creature enters, create a 1/1 white and blue Merfolk creature token.")],
    ..oracle::MERROW_SKYSWIMMER
};

pub static EXTRA: &[&CardDef] = &[&MERFOLK];
