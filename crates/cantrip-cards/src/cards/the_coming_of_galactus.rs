use cantrip_api::*;
use cantrip_mechanics::effects::{self, destroy_first_target};
use cantrip_mechanics::{targets, tokens, triggers};
use cantrip_oracle as oracle;

/// Galactus, a legendary 16/16 black Elder Alien creature token with flying, trample, and "Whenever Galactus attacks,
/// destroy target land" (CR 205.4a, 111.4).
pub static GALACTUS: CardDef = CardDef {
    name: "Galactus",
    has_cost: false,
    colors: Some(ColorSet::of(Color::Black)),
    types: TypeSet::of(&[CardType::Creature]),
    supertypes: Supertypes { basic: false, legendary: true, snow: false, world: false },
    subtypes: SubtypeSet::of(&[Subtype::Elder, Subtype::Alien]),
    power: Some(16),
    toughness: Some(16),
    keywords: Keywords::of(&[Keyword::Flying, Keyword::Trample]),
    abilities: &[triggered(triggers::THIS_ATTACKS, destroy_first_target, "Whenever Galactus attacks, destroy target land.").targets(&[targets::LAND])],
    ..CardDef::BLANK
};

pub static EXTRA: &[&CardDef] = &[&GALACTUS];

/// IV: "Create Galactus."
fn create_galactus(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    tokens::create(g, item.controller, &GALACTUS, 1)?;
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        chapter(1, destroy_first_target, "I \u{2014} Destroy up to one target nonland permanent.").targets(&[up_to_one(targets::NONLAND_PERMANENT.filter)]),
        chapter(2, effects::each_opponent_loses_2_life, "II \u{2014} Each opponent loses 2 life."),
        chapter(3, effects::each_opponent_loses_2_life, "III \u{2014} Each opponent loses 2 life."),
        chapter(
            4,
            create_galactus,
            "IV \u{2014} Create Galactus, a legendary 16/16 black Elder Alien creature token with flying, trample, and \"Whenever Galactus attacks, destroy target land.\"",
        ),
    ],
    ..oracle::THE_COMING_OF_GALACTUS
};
