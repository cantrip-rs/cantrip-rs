use cantrip_api::*;
use cantrip_mechanics::{blight, effects};
use cantrip_oracle as oracle;

/// "This creature deals 2 damage to each opponent" (CR 120.3).
fn sting(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    effects::damage_each_opponent(g, item, 2);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[ability(
        cost("{1}{R}").and(TAP).and(blight::cost(1)),
        sting,
        "{1}{R}, {T}, Blight 1: This creature deals 2 damage to each opponent.",
    )],
    ..oracle::STING_SLINGER
};
