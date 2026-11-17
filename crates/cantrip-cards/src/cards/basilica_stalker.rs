use cantrip_api::*;
use cantrip_mechanics::{disguise, effects, triggers};
use cantrip_oracle as oracle;

/// "You gain 1 life and surveil 1." (CR 119.1, 701.25a): the life first, then the surveil, each its own instruction.
fn gain_1_life_and_surveil_1(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    effects::gain_life(g, item, 1)?;
    effects::surveil(g, item, 1)
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        triggers::DEALS_COMBAT_DAMAGE_TO_PLAYER,
        gain_1_life_and_surveil_1,
        "Whenever this creature deals combat damage to a player, you gain 1 life and surveil 1.",
    )],
    ways: &[disguise::way(Cost::mana(mana("{4}{B}")), "Disguise {4}{B}")],
    ..oracle::BASILICA_STALKER
};
