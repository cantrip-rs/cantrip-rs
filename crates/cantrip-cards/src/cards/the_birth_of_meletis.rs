use cantrip_api::*;
use cantrip_mechanics::{effects, tokens};
use cantrip_oracle as oracle;

/// The 0/4 colorless Wall artifact creature token with defender; The Birth of Meletis is the only printed card that makes it.
pub static WALL: CardDef = CardDef {
    colors: Some(ColorSet::NONE),
    types: TypeSet::of(&[CardType::Artifact, CardType::Creature]),
    power: Some(0),
    toughness: Some(4),
    keywords: Keywords::of(&[Keyword::Defender]),
    text: "Defender",
    ..token!(Wall)
};

/// "II — Create a 0/4 colorless Wall artifact creature token with defender." (CR 111.1).
fn ii_create_a_wall(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    tokens::create(g, item.controller, &WALL, 1)?;
    Ok(())
}

/// "III — You gain 2 life." (CR 119.3).
fn iii_gain_2_life(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    g.gain_life(item.controller, 2);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        chapter(
            1,
            effects::search_for_a_basic_plains_card,
            "I — Search your library for a basic Plains card, reveal it, put it into your hand, then shuffle.",
        ),
        chapter(
            2,
            ii_create_a_wall,
            "II — Create a 0/4 colorless Wall artifact creature token with defender.",
        ),
        chapter(3, iii_gain_2_life, "III — You gain 2 life."),
    ],
    ..oracle::THE_BIRTH_OF_MELETIS
};

pub static EXTRA: &[&CardDef] = &[&WALL];
