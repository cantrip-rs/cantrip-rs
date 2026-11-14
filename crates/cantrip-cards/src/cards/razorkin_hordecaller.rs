use cantrip_api::*;
use cantrip_mechanics::tokens;
use cantrip_mechanics::triggers::YOU_ATTACK;
use cantrip_oracle as oracle;

/// The 1/1 red Gremlin creature token (CR 111.4).
pub static GREMLIN: CardDef = CardDef {
    colors: Some(ColorSet::of(Color::Red)),
    types: TypeSet::of(&[CardType::Creature]),
    power: Some(1),
    toughness: Some(1),
    ..token!(Gremlin)
};

fn create_a_gremlin(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    tokens::create(g, item.controller, &GREMLIN, 1)?;
    Ok(())
}

pub static EXTRA: &[&CardDef] = &[&GREMLIN];

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(YOU_ATTACK, create_a_gremlin, "Whenever you attack, create a 1/1 red Gremlin creature token.")],
    ..oracle::RAZORKIN_HORDECALLER
};
