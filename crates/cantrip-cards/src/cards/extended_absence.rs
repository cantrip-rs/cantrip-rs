use cantrip_api::*;
use cantrip_mechanics::{effects, exile_target, targets};
use cantrip_oracle as oracle;

/// "Exile target creature or planeswalker. Extended Absence deals 1 damage to each opponent and you gain 1
/// life."
fn effect(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    exile_target(g, item, 0);
    effects::damage_each_opponent_and_gain_life(g, item, 1)
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[targets::CREATURE_OR_PLANESWALKER], effect),
    ..oracle::EXTENDED_ABSENCE
};
