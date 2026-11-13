use cantrip_api::*;
use cantrip_oracle as oracle;
use cantrip_mechanics::tokens;

pub static MINOR_DEMON: CardDef = CardDef {
    name: "Minor Demon",
    has_cost: false,
    types: TypeSet::of(&[CardType::Creature]),
    subtypes: SubtypeSet::of(&[Subtype::Demon]),
    colors: Some(ColorSet(12)),
    power: Some(1),
    toughness: Some(1),
    ..CardDef::BLANK
};

/// "{2}{B}{R}, {T}: Create a 1/1 black and red Demon creature token named Minor Demon." (CR 608.2).
fn create_1_1_black_red(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    tokens::create(g, item.controller, &MINOR_DEMON, 1)?;
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        ability(
            cost("{2}{B}{R}").and(TAP),
            create_1_1_black_red,
            "{2}{B}{R}, {T}: Create a 1/1 black and red Demon creature token named Minor Demon.",
        ),
    ],
    ..oracle::BORIS_DEVILBOON
};

pub static EXTRA: &[&CardDef] = &[&MINOR_DEMON];
