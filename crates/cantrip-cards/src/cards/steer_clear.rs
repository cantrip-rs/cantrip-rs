use cantrip_api::*;
use cantrip_mechanics::{damage_target, history, lands, targets};
use cantrip_oracle as oracle;

/// "Steer Clear deals 2 damage to target attacking or blocking creature. Steer Clear deals 4 damage to that creature instead if you
/// controlled a Mount as you cast this spell." Whether you did is what the board was as the spell became cast (CR 601.2i), kept in
/// its cast record: a Mount lost since still counts, one gained since does not (the rulings). The target is judged as it resolves
/// (CR 608.2b); the damage is 4 instead of 2, never both.
fn effect(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let amount = if g.history().fact_as_cast(item.object) != 0 { 4 } else { 2 };
    damage_target(g, item, 0, amount);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[targets::ATTACKING_OR_BLOCKING_CREATURE], effect),
    cast_fact: Some(history::controlled_a_as_cast::<{ lands::sub(Subtype::Mount) }>),
    ..oracle::STEER_CLEAR
};
