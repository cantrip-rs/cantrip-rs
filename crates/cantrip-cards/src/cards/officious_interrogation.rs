use cantrip_api::*;
use cantrip_mechanics::{creatures_you_control, targets, tokens};
use cantrip_oracle as oracle;

/// "This spell costs {W}{U} more to cast for each target beyond the first" (CR 601.2f): read once the targets are announced
/// (CR 601.2c), so zero targets or one costs nothing more. Its mana value is its mana cost's alone (CR 202.3).
fn more_per_extra_target(_: &dyn Query, _: ObjectId, _: PlayerId, _: Costed, targets: &[Target]) -> CostDelta {
    let chosen = targets.iter().filter(|t| **t != Target::None).count();
    CostDelta::more_mana(mana("{W}{U}").times(chosen.saturating_sub(1) as u32).unwrap_or(ManaCost::ZERO))
}

/// "Choose any number of target players. Investigate X times, where X is the total number of creatures those players control" (CR 115.1,
/// 701.16a, 608.2b): X is counted as the spell resolves, over the chosen players that are still legal targets, and Interrogation's controller
/// investigates that many times (a Clue each, one at a time). With every target gone it does nothing (CR 608.2b); with none chosen X is 0.
fn investigate_for_creatures_they_control(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let players: Vec<PlayerId> =
        (0..item.target_count as usize).filter(|&slot| g.target_legal(item, slot)).filter_map(|slot| g.state().target(item, slot).player()).collect();
    let x: usize = players.into_iter().map(|player| creatures_you_control(g, player)).sum();
    for _ in 0..x {
        tokens::investigate(g, item.controller)?;
    }
    Ok(())
}

pub static EXTRA: &[&CardDef] = &[&tokens::CLUE];

pub static CARD: CardDef = CardDef {
    spell: spell(&[targets::PLAYER.between(0, TargetSpec::ANY_NUMBER)], investigate_for_creatures_they_control),
    own_cost_changes: &[more_per_extra_target],
    ..oracle::OFFICIOUS_INTERROGATION
};
