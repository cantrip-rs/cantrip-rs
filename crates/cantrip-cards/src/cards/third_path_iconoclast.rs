use cantrip_api::*;
use cantrip_mechanics::tokens;
use cantrip_oracle as oracle;

/// The 1/1 colorless Soldier artifact creature token it makes.
pub static SOLDIER: CardDef = CardDef {
    name: "Soldier Token",
    token_name: Some(&[Subtype::Soldier]),
    has_cost: false,
    colors: Some(ColorSet::NONE),
    types: TypeSet::of(&[CardType::Artifact, CardType::Creature]),
    subtypes: SubtypeSet::of(&[Subtype::Soldier]),
    power: Some(1),
    toughness: Some(1),
    ..CardDef::BLANK
};

fn soldier(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    tokens::create(g, item.controller, &SOLDIER, 1)?;
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(cantrip_mechanics::triggers::NONCREATURE_SPELL_YOU_CAST, soldier, "Whenever you cast a noncreature spell, create a 1/1 colorless Soldier artifact creature token.")],
    ..oracle::THIRD_PATH_ICONOCLAST
};

pub static EXTRA: &[&CardDef] = &[&SOLDIER];
