use cantrip_api::*;
use cantrip_mechanics::{exile_target, targets, triggers};
use cantrip_oracle as oracle;

/// "Choose up to one target creature. If it's suspected, exile it. Otherwise, suspect it." (CR 701.13a, 701.60a, 608.2b): whether it is suspected is
/// looked at as the ability resolves; a target no longer legal then, or none chosen, means nothing happens.
fn exile_if_suspected_otherwise_suspect(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(creature) = targets::object(g, item, 0) else { return Ok(()) };
    if g.is_suspected(creature) {
        exile_target(g, item, 0);
    } else {
        g.suspect(creature);
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            triggers::THIS_ENTERS_OR_ATTACKS,
            exile_if_suspected_otherwise_suspect,
            "Whenever Agrus Kos enters or attacks, choose up to one target creature. If it's suspected, exile it. Otherwise, suspect it. (A suspected creature has menace and can't block.)",
        )
        .targets(&[targets::CREATURE.optional()]),
    ],
    ..oracle::AGRUS_KOS_SPIRIT_OF_JUSTICE
};
