use cantrip_api::*;
use cantrip_mechanics::{damage_target, effects, targets};
use cantrip_oracle as oracle;

/// "Iroh's Demonstration deals 1 damage to each creature your opponents control."
fn opponents_creatures(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    effects::damage_each_creature_opponents_control(g, item, 1);
    Ok(())
}

/// "Iroh's Demonstration deals 4 damage to target creature."
fn four_damage(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    damage_target(g, item, 0, 4);
    Ok(())
}

/// Choose one (CR 700.2).
static MODAL: ModalDef = modes(&[
    mode(opponents_creatures, "Iroh's Demonstration deals 1 damage to each creature your opponents control."),
    mode(four_damage, "Iroh's Demonstration deals 4 damage to target creature.").targets(&[targets::CREATURE]),
])
.choose(1, 1);

pub static CARD: CardDef = CardDef { spell: Some(SpellDef::MODAL), modal: Some(&MODAL), ..oracle::IROHS_DEMONSTRATION };
