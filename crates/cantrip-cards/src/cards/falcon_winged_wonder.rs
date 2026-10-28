use cantrip_api::*;
use cantrip_mechanics::{effects, tokens, triggers};
use cantrip_oracle as oracle;

/// "Whenever Redwing attacks, surveil 1." (CR 701.25a): its controller looks at the top card of their library and may put it
/// into their graveyard.
fn surveil_one(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    effects::surveil(g, item, 1)
}

/// Redwing, a legendary 1/1 blue Bird Scout creature token with flying and "Whenever Redwing attacks, surveil 1." (CR 111.4,
/// 205.4a: the token is named by the text that makes it).
pub static REDWING: CardDef = CardDef {
    name: "Redwing",
    has_cost: false,
    colors: Some(ColorSet::of(Color::Blue)),
    types: TypeSet::of(&[CardType::Creature]),
    supertypes: Supertypes { basic: false, legendary: true, snow: false },
    subtypes: SubtypeSet::of(&[Subtype::Bird, Subtype::Scout]),
    power: Some(1),
    toughness: Some(1),
    keywords: Keywords::of(&[Keyword::Flying]),
    abilities: &[triggered(triggers::THIS_ATTACKS, surveil_one, "Whenever Redwing attacks, surveil 1.")],
    ..CardDef::BLANK
};

/// "Create Redwing, a legendary 1/1 blue Bird Scout creature token with flying and 'Whenever Redwing attacks, surveil 1.'"
fn create_redwing(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    tokens::create(g, item.controller, &REDWING, 1)?;
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        triggers::THIS_ENTERS,
        create_redwing,
        "Avian Telepathy \u{2014} When Falcon enters, create Redwing, a legendary 1/1 blue Bird Scout creature token with flying and \"Whenever Redwing attacks, surveil 1.\"",
    )],
    ..oracle::FALCON_WINGED_WONDER
};

pub static EXTRA: &[&CardDef] = &[&REDWING];
