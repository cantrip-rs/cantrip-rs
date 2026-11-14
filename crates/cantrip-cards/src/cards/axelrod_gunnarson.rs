use cantrip_api::*;
use cantrip_mechanics::{targets, triggers};
use cantrip_oracle as oracle;

/// Gain 1 life, then deal 1 damage (CR 608.2c, 119.3, 120.3).
fn gain_life_and_damage(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    g.gain_life(item.controller, 1);
    g.damage(item.object, item.target(0), 1);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            triggers::CREATURE_DEALT_DAMAGE_BY_THIS_DIES,
            gain_life_and_damage,
            "Whenever a creature dealt damage by Axelrod Gunnarson this turn dies, you gain 1 life and Axelrod Gunnarson deals 1 damage to target player or planeswalker.",
        ).targets(&[targets::PLAYER_OR_PLANESWALKER]),
    ],
    ..oracle::AXELROD_GUNNARSON
};
