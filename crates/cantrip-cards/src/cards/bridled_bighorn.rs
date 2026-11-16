use cantrip_api::*;
use cantrip_mechanics::saddle;
use cantrip_mechanics::tokens;
use cantrip_mechanics::triggers::THIS_ATTACKS_WHILE_SADDLED;
use cantrip_oracle as oracle;

/// The 1/1 white Sheep creature token (CR 111.1) Bridled Bighorn makes.
pub static SHEEP: CardDef = CardDef {
    colors: Some(ColorSet::of(Color::White)),
    types: TypeSet::of(&[CardType::Creature]),
    power: Some(1),
    toughness: Some(1),
    ..token!(Sheep)
};

/// "Create a 1/1 white Sheep creature token." (CR 111.1, 701.7a)
fn create_a_sheep(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    tokens::create(g, item.controller, &SHEEP, 1)?;
    Ok(())
}

/// Vigilance is a core keyword (CR 702.20), from the card's characteristics. "Saddle 2" (CR 702.171a) is the shared mechanic, and
/// "attacks while saddled" is judged when the Bighorn attacks (CR 702.171b).
pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(THIS_ATTACKS_WHILE_SADDLED, create_a_sheep, "Whenever this creature attacks while saddled, create a 1/1 white Sheep creature token."),
        saddle::ability(2, "Saddle 2 (Tap any number of other creatures you control with total power 2 or more: This Mount becomes saddled until end of turn. Saddle only as a sorcery.)"),
    ],
    ..oracle::BRIDLED_BIGHORN
};

pub static EXTRA: &[&CardDef] = &[&SHEEP];
