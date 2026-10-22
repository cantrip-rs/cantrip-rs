use cantrip_api::*;
use cantrip_mechanics::tokens;
use cantrip_oracle as oracle;

/// The 1/1 white Warrior creature token (CR 111.4).
pub static WARRIOR: CardDef = CardDef {
    name: "Warrior Token",
    token_name: Some(&[Subtype::Warrior]),
    has_cost: false,
    colors: Some(ColorSet::of(Color::White)),
    types: TypeSet::of(&[CardType::Creature]),
    subtypes: SubtypeSet::of(&[Subtype::Warrior]),
    power: Some(1),
    toughness: Some(1),
    ..CardDef::BLANK
};

/// "Create X 1/1 white Warrior creature tokens." X is the number chosen, not the mana actually spent (CR 107.3a), and the
/// tokens are created together, as one event (CR 111.1).
fn effect(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    tokens::create(g, item.controller, &WARRIOR, item.x as u32)?;
    Ok(())
}

pub static EXTRA: &[&CardDef] = &[&WARRIOR];

pub static CARD: CardDef = CardDef { spell: spell(&[], effect), ..oracle::SECURE_THE_WASTES };
