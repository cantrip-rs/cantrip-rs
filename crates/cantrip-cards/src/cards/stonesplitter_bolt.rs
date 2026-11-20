use cantrip_api::*;
use cantrip_mechanics::{bargain, damage_target, targets};
use cantrip_oracle as oracle;

/// "Stonesplitter Bolt deals X damage to target creature or planeswalker. If this spell was bargained, it deals twice X damage to
/// that permanent instead." (CR 107.3a, 120.3, 702.166b): the X announced as it is cast, doubled when bargained; a copy of a
/// bargained spell is bargained (CR 707.10). Nothing when the one target is no longer legal (CR 608.2b).
fn bolt(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    damage_target(g, item, 0, if bargain::bargained(item) { item.x * 2 } else { item.x });
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[targets::CREATURE_OR_PLANESWALKER], bolt),
    optional_cost: Some(bargain::OPTIONAL_COST),
    labels: bargain::LABELS,
    ..oracle::STONESPLITTER_BOLT
};
