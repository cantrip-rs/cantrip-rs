use cantrip_api::*;
use cantrip_mechanics::{targets, tokens, triggers::{self, THIS_ENTERS}};
use cantrip_oracle as oracle;

/// A 5/3 green Elemental creature token.
pub static ELEMENTAL: CardDef = CardDef {
    colors: Some(ColorSet::of(Color::Green)),
    types: TypeSet::of(&[CardType::Creature]),
    power: Some(5),
    toughness: Some(3),
    ..token!(Elemental)
};

/// "Return target land card from your graveyard to the battlefield", if it is still a legal target.
fn return_land(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(land) = targets::object(g, item, 0) else { return Ok(()) };
    g.put_onto_battlefield(land, item.controller, false)?;
    Ok(())
}

fn create_elemental(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    tokens::create(g, item.controller, &ELEMENTAL, 1)?;
    Ok(())
}

pub static EXTRA: &[&CardDef] = &[&ELEMENTAL];

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(THIS_ENTERS, return_land, "When Titania, Protector of Argoth enters, return target land card from your graveyard to the battlefield.")
            .targets(&[targets::LAND_CARD_IN_YOUR_GRAVEYARD]),
        triggered(when_asked(EventMask::ZONE_CHANGE, triggers::type_you_control_to_graveyard::<{ CardType::Land as u8 }>), create_elemental, "Whenever a land you control is put into a graveyard from the battlefield, create a 5/3 green Elemental creature token."),
    ],
    ..oracle::TITANIA_PROTECTOR_OF_ARGOTH
};
