use cantrip_api::*;
use cantrip_mechanics::statics::{grant_delayed_ability, your_creatures};
use cantrip_mechanics::triggers::THIS_ENTERS;
use cantrip_mechanics::tokens;
use cantrip_oracle as oracle;

/// Murf, the legendary 0/1 green Ooze Slug token Rok-Tahk makes, with indestructible (CR 111.1, 111.4, 205.4a, 702.12b).
pub static MURF: CardDef = CardDef {
    name: "Murf",
    has_cost: false,
    colors: Some(ColorSet::of(Color::Green)),
    types: TypeSet::of(&[CardType::Creature]),
    supertypes: Supertypes { basic: false, legendary: true, snow: false, world: false },
    subtypes: SubtypeSet::of(&[Subtype::Ooze, Subtype::Slug]),
    power: Some(0),
    toughness: Some(1),
    keywords: Keywords::of(&[Keyword::Indestructible]),
    ..CardDef::BLANK
};

fn create_murf(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    tokens::create(g, item.controller, &MURF, 1)?;
    Ok(())
}

/// "Creatures you control have '{T}: Add {G}{G}.'" (CR 613.1f, layer 6): a mana ability each of them is given, the creature
/// itself its source (CR 113.7); Rok-Tahk is a creature too, so it has it while it is on the battlefield.
pub static CARD: CardDef = CardDef {
    delayed: &[mana_ability(TAP, add_n(Mana::G, 2), "{T}: Add {G}{G}.")],
    statics: &[static_ability(
        Layer::Ability,
        your_creatures,
        grant_delayed_ability,
        "Creatures you control have \"{T}: Add {G}{G}.\"",
    )],
    abilities: &[triggered(
        THIS_ENTERS,
        create_murf,
        "When Rok-Tahk enters, create Murf, a legendary 0/1 green Ooze Slug creature token with indestructible.",
    )],
    ..oracle::ROK_TAHK_BUDDING_SCIENTIST
};

pub static EXTRA: &[&CardDef] = &[&MURF];
