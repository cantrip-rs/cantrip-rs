use cantrip_api::*;
use cantrip_mechanics::{targets, triggers};
use cantrip_oracle as oracle;

use super::hapatra_the_desert_frost::creature_the_slots_opponent_controls;

/// "... put X -1/-1 counters on up to one target creature that player controls, where X is the greatest mana value among
/// cards in your graveyard": X is read as the ability resolves (CR 608.2h), whatever was targeted, and each target still
/// legal gets X counters (CR 122.1).
fn effect(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let x = g
        .state()
        .zone(Zone::graveyard(item.controller))
        .iter()
        .map(|&id| g.mana_value(id))
        .max()
        .unwrap_or(0)
        .min(i16::MAX as u32) as i16;
    let puts: Vec<(ObjectId, CounterKind, i64)> = targets::legal_objects(g, item).into_iter().map(|id| (id, CounterKind::MinusOne, i64::from(x))).collect();
    g.add_counters_together(&puts);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        triggers::THIS_ENTERS,
        effect,
        "When Hapatra enters, for each opponent, put X -1/-1 counters on up to one target creature that player controls, where X is the greatest mana value among cards in your graveyard.",
    )
    .targets(&[target(creature_the_slots_opponent_controls).between(0, TargetSpec::EACH_OPPONENT)])],
    ..oracle::HAPATRA_THE_DESERT_FANG
};
