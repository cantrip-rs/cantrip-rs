use cantrip_api::*;
use cantrip_mechanics::{damage_target, flashback, targets};
use cantrip_oracle as oracle;

/// "Tome Blast deals 2 damage to any target." A target that is illegal as it resolves is not dealt damage (CR 608.2b).
fn effect(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    damage_target(g, item, 0, 2);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[targets::ANY], effect),
    ways: &[flashback::way(cost("{4}{R}"), "Flashback {4}{R}")],
    ..oracle::TOME_BLAST
};
