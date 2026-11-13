use cantrip_api::*;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

/// "Joust Through deals 3 damage to target attacking or blocking creature. You gain 1 life." The only target is checked when the
/// spell resolves (CR 608.2b): if it is no longer legal the spell does not resolve at all and no life is gained.
fn effect(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    effects::damage_target_and_gain_life(g, item, 3, 1)
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[targets::ATTACKING_OR_BLOCKING_CREATURE], effect),
    ..oracle::JOUST_THROUGH
};
