use cantrip_api::*;
use cantrip_mechanics::{effects, tokens};
use cantrip_oracle as oracle;

/// The 4/4 red Dinosaur Dragon creature token with flying (only this card prints it).
pub static DINOSAUR_DRAGON: CardDef = CardDef {
    colors: Some(ColorSet::of(Color::Red)),
    types: TypeSet::of(&[CardType::Creature]),
    power: Some(4),
    toughness: Some(4),
    keywords: Keywords::of(&[Keyword::Flying]),
    text: "Flying",
    ..token!(Dinosaur Dragon)
};

/// "Other creatures you control gain haste until end of turn. Put a +1/+1 counter on this creature." (CR 611.2, 122.1): the
/// creatures it gives haste to are those it finds as it resolves, itself excluded.
fn others_gain_haste_and_this_grows(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    effects::other_creatures_you_control_gain(g, item, &[Keyword::Haste]);
    effects::plus_one_counter_on_this(g, item)
}

/// "Create a 4/4 red Dinosaur Dragon creature token with flying." (CR 111.1, 701.7).
fn dinosaur_dragon(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    tokens::create(g, item.controller, &DINOSAUR_DRAGON, 1)?;
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        ability(
            cost("{R}"),
            others_gain_haste_and_this_grows,
            "Exhaust \u{2014} {R}: Other creatures you control gain haste until end of turn. Put a +1/+1 counter on this creature. (Activate each exhaust ability only once.)",
        )
        .exhaust(),
        ability(
            cost("{3}{R}"),
            dinosaur_dragon,
            "Exhaust \u{2014} {3}{R}: Create a 4/4 red Dinosaur Dragon creature token with flying. (Activate each exhaust ability only once.)",
        )
        .exhaust(),
    ],
    ..oracle::DRACONAUTICS_ENGINEER
};

pub static EXTRA: &[&CardDef] = &[&DINOSAUR_DRAGON];
