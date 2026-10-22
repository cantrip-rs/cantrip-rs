use cantrip_api::*;
use cantrip_mechanics::targets::filters;
use cantrip_mechanics::{tokens, triggers};
use cantrip_oracle as oracle;

/// A 4/4 red Scorpion Dragon creature token with flying and haste.
pub static SCORPION_DRAGON: CardDef = CardDef {
    name: "Scorpion Dragon Token",
    token_name: Some(&[Subtype::Scorpion, Subtype::Dragon]),
    has_cost: false,
    colors: Some(ColorSet::of(Color::Red)),
    types: TypeSet::of(&[CardType::Creature]),
    subtypes: SubtypeSet::of(&[Subtype::Scorpion, Subtype::Dragon]),
    power: Some(4),
    toughness: Some(4),
    keywords: Keywords::of(&[Keyword::Flying, Keyword::Haste]),
    text: "Flying, haste",
    ..CardDef::BLANK
};

/// "Create a tapped Treasure token."
fn tapped_treasure(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    tokens::create_tapped_treasure(g, item.controller)
}

/// "Create a 4/4 red Scorpion Dragon creature token with flying and haste."
fn scorpion_dragon(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    tokens::create(g, item.controller, &SCORPION_DRAGON, 1)?;
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        // CR 700.13: once for each spell or ability that is a crime; "triggers only once each turn" on top of that.
        triggered(triggers::YOU_COMMIT_A_CRIME, tapped_treasure, "Whenever you commit a crime, create a tapped Treasure token. This ability triggers only once each turn. (Targeting opponents, anything they control, and/or cards in their graveyards is a crime.)")
            .triggers_each_turn(1),
        ability(sacrifice(3, filters::treasure), scorpion_dragon, "Sacrifice three Treasures: Create a 4/4 red Scorpion Dragon creature token with flying and haste. Activate only as a sorcery.")
            .sorcery_speed(),
    ],
    ..oracle::MAGDA_THE_HOARDMASTER
};

pub static EXTRA: &[&CardDef] = &[&tokens::TREASURE, &SCORPION_DRAGON];
