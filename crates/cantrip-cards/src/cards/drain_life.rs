use cantrip_api::*;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

/// "Drain Life deals X damage to any target. You gain life equal to the damage dealt, but not more life than the player's
/// life total before the damage was dealt, the planeswalker's loyalty before the damage was dealt, or the creature's
/// toughness." (`effects::deal_damage_for_drained_life`).
fn effect(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let life = effects::deal_damage_for_drained_life(g, item.object, item.target(0), u32::from(item.x));
    g.gain_life(item.controller, life as i32);
    Ok(())
}

pub static CARD: CardDef = CardDef { spell: spell(&[targets::ANY], effect), x_mana: XMana::only(Mana::B), ..oracle::DRAIN_LIFE };
