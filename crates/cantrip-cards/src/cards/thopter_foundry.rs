use cantrip_api::*;
use cantrip_mechanics::{targets, tokens};
use cantrip_oracle as oracle;

/// A 1/1 blue Thopter artifact creature token with flying.
pub static THOPTER: CardDef = CardDef {
    name: "Thopter Token",
    token_name: Some(&[Subtype::Thopter]),
    has_cost: false,
    colors: Some(ColorSet::of(Color::Blue)),
    types: TypeSet::of(&[CardType::Artifact, CardType::Creature]),
    subtypes: SubtypeSet::of(&[Subtype::Thopter]),
    power: Some(1),
    toughness: Some(1),
    keywords: Keywords::of(&[Keyword::Flying]),
    ..CardDef::BLANK
};

fn make_thopter(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    tokens::create(g, item.controller, &THOPTER, 1)?;
    g.gain_life(item.controller, 1);
    Ok(())
}

pub static EXTRA: &[&CardDef] = &[&THOPTER];

pub static CARD: CardDef = CardDef {
    abilities: &[ability(cost("{1}").and(sacrifice(1, targets::filters::nontoken_artifact)), make_thopter, "{1}, Sacrifice a nontoken artifact: Create a 1/1 blue Thopter artifact creature token with flying. You gain 1 life.")],
    ..oracle::THOPTER_FOUNDRY
};
