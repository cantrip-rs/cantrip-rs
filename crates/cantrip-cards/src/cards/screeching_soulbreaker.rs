use cantrip_api::*;
use cantrip_mechanics::{effects, triggers::THIS_ATTACKS};
use cantrip_oracle as oracle;

/// Flying is a core keyword read from its characteristics. "Whenever this creature attacks, it deals 1 damage to each
/// opponent and you gain 1 life." One damage event for each opponent still in the game (CR 120.2, 102.2), then the
/// controller gains 1 life (CR 119.3).
fn drain(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    effects::damage_each_opponent_and_gain_life(g, item, 1)
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(THIS_ATTACKS, drain, "Whenever this creature attacks, it deals 1 damage to each opponent and you gain 1 life.")],
    ..oracle::SCREECHING_SOULBREAKER
};
