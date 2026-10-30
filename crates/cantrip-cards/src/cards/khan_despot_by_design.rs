use cantrip_api::*;
use cantrip_mechanics::{effects, triggers};
use cantrip_oracle as oracle;

/// "When Khan enters, he deals 1 damage to each opponent and each creature your opponents control" (CR 120.2, 608.2c): one
/// damage event for the opponents still in the game and every creature those opponents control as the trigger resolves,
/// all dealt together; Khan is the source, as he last was if he has left (CR 113.7a).
fn one_apiece(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    effects::damage_each_opponent_and_each_creature_they_control(g, item, 1);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        triggers::THIS_ENTERS,
        one_apiece,
        "When Khan enters, he deals 1 damage to each opponent and each creature your opponents control.",
    )],
    ..oracle::KHAN_DESPOT_BY_DESIGN
};
