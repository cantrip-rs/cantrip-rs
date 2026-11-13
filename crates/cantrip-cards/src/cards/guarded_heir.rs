use cantrip_api::*;
use cantrip_mechanics::{tokens, triggers::THIS_ENTERS};
use cantrip_oracle as oracle;

/// The 3/3 white Knight creature token Guarded Heir makes (CR 111.4).
pub static KNIGHT_3_3: CardDef = CardDef {
    colors: Some(ColorSet::of(Color::White)),
    types: TypeSet::of(&[CardType::Creature]),
    power: Some(3),
    toughness: Some(3),
    ..token!(Knight)
};

/// "When this creature enters, create two 3/3 white Knight creature tokens." Both tokens are made together under the ability's
/// controller (CR 111.2).
fn create_two_knights(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    tokens::create(g, item.controller, &KNIGHT_3_3, 2)?;
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(THIS_ENTERS, create_two_knights, "When this creature enters, create two 3/3 white Knight creature tokens.")],
    ..oracle::GUARDED_HEIR
};

pub static EXTRA: &[&CardDef] = &[&KNIGHT_3_3];
