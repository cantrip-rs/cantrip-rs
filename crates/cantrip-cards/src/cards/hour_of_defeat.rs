use cantrip_api::*;
use cantrip_mechanics::{destroy_target, effects, targets};
use cantrip_oracle as oracle;

/// "Destroy target creature. Surveil 1." (CR 701.8, 701.25): the surveil happens whether or not the creature was destroyed
/// (an indestructible one stays), but not when the spell is countered on resolution for having an illegal target (CR 608.2b).
fn effect(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    destroy_target(g, item, 0);
    g.end_sentence();
    effects::surveil(g, item, 1)
}

pub static CARD: CardDef = CardDef { spell: spell(&[targets::CREATURE], effect), ..oracle::HOUR_OF_DEFEAT };
