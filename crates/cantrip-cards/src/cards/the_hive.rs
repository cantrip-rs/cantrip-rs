use cantrip_api::*;
use cantrip_oracle as oracle;
use cantrip_mechanics::tokens;

pub static WASP: CardDef = CardDef {
    name: "Wasp",
    has_cost: false,
    types: TypeSet::of(&[CardType::Artifact, CardType::Creature]),
    subtypes: SubtypeSet::of(&[Subtype::Insect]),
    power: Some(1),
    toughness: Some(1),
    keywords: Keywords::of(&[Keyword::Flying]),
    ..CardDef::BLANK
};

/// "{5}, {T}: Create a 1/1 colorless Insect artifact creature token with flying named Wasp." (CR 608.2).
fn create_1_1_colorless_insect(
    g: &mut dyn Game,
    item: &StackItem,
) -> Result<(), Suspend> {
    tokens::create(g, item.controller, &WASP, 1)?;
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        ability(
            cost("{5}").and(TAP),
            create_1_1_colorless_insect,
            "{5}, {T}: Create a 1/1 colorless Insect artifact creature token with flying named Wasp.",
        ),
    ],
    ..oracle::THE_HIVE
};

pub static EXTRA: &[&CardDef] = &[&WASP];
