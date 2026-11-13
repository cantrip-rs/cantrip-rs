use cantrip_api::*;
use cantrip_mechanics::targets;
use cantrip_oracle as oracle;

/// "Create a token that's a copy of target creature. If this spell was kicked, create five of those tokens instead." (CR 707.2,
/// 111.1): the copiable values of the target as the spell resolves, one token or, with the kicker paid (`item.kicked`), five of
/// them as one instruction (they all enter together, CR 603.2c). Nothing happens if the target is no longer legal (CR 608.2b).
fn effect(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(original) = targets::object(g, item, 0) else { return Ok(()) };
    let count = if item.kicked > 0 { 5 } else { 1 };
    g.create_token_copies(original, item.controller, count)?;
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[targets::CREATURE], effect),
    kicker: kicker(mana("{5}")),
    ..oracle::RITE_OF_REPLICATION
};
