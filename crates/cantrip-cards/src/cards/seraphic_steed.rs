use cantrip_api::*;
use cantrip_mechanics::triggers::THIS_ATTACKS_WHILE_SADDLED;
use cantrip_mechanics::{saddle, tokens};
use cantrip_oracle as oracle;

/// The 3/3 white Angel creature token with flying.
pub static ANGEL: CardDef = CardDef {
    colors: Some(ColorSet::of(Color::White)),
    types: TypeSet::of(&[CardType::Creature]),
    power: Some(3),
    toughness: Some(3),
    keywords: Keywords::of(&[Keyword::Flying]),
    text: "Flying",
    ..token!(Angel)
};

fn create_an_angel(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    tokens::create(g, item.controller, &ANGEL, 1)?;
    Ok(())
}

/// "Saddle 4" (CR 702.171a) and "attacks while saddled" are the shared mechanics.
pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            THIS_ATTACKS_WHILE_SADDLED,
            create_an_angel,
            "Whenever this creature attacks while saddled, create a 3/3 white Angel creature token with flying.",
        ),
        saddle::ability(4, "Saddle 4 (Tap any number of other creatures you control with total power 4 or more: This Mount becomes saddled until end of turn. Saddle only as a sorcery.)"),
    ],
    ..oracle::SERAPHIC_STEED
};

pub static EXTRA: &[&CardDef] = &[&ANGEL];
