use cantrip_api::*;
use cantrip_mechanics::{targets, tokens};
use cantrip_oracle as oracle;

/// "Torch the Witness deals twice X damage to target creature. If excess damage was dealt to that creature this way, investigate."
/// The X announced as it is cast (CR 601.2b) and doubled (CR 107.3a). Excess damage is the damage beyond lethal damage, damage
/// already marked on the creature counted (CR 120.10), judged by the damage procedure after any replacement or prevention; no
/// Clue when the creature is gone or the damage was prevented (CR 608.2b).
fn torch(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let target = item.target(0);
    if let Some(result) = g.damage_result(item.object, target, u32::from(item.x) * 2)
        && result.target == target
    {
        g.end_sentence();
        if result.excess > 0 {
            tokens::you_investigate(g, item)?;
        }
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[targets::CREATURE], torch),
    ..oracle::TORCH_THE_WITNESS
};

pub static EXTRA: &[&CardDef] = &[&tokens::CLUE];
