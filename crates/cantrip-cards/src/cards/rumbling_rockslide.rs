use cantrip_api::*;
use cantrip_mechanics::{damage_target, lands_you_control, targets};
use cantrip_oracle as oracle;

/// "Rumbling Rockslide deals damage to target creature equal to the number of lands you control": counted as this resolves
/// (CR 608.2h), a land the caster controls as of then. With the creature no longer a legal target nothing happens (CR 608.2b).
fn effect(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let lands = lands_you_control(g, item.controller) as u32;
    damage_target(g, item, 0, lands);
    Ok(())
}

pub static CARD: CardDef = CardDef { spell: spell(&[targets::CREATURE], effect), ..oracle::RUMBLING_ROCKSLIDE };
