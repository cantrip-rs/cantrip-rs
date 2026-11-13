use cantrip_api::*;
use cantrip_mechanics::{pump, triggers};
use cantrip_oracle as oracle;

/// Count the creatures still blocking this creature as the ability resolves (CR 608.2h, 509.1h).
fn weakened_by_blockers(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let blockers = g.state().blocks.iter().filter(|b| b.attacker == item.object).count();
    let beyond_first = if blockers > 1 { (blockers - 1) as i32 } else { 0 };
    pump(g, item.object, item.object, -2 * beyond_first, -beyond_first);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            when_asked(EventMask::BECAME_BLOCKED, triggers::this_blocks_or_becomes_blocked),
            weakened_by_blockers,
            "Whenever this creature becomes blocked, it gets -2/-1 until end of turn for each creature blocking it beyond the first.",
        ),
    ],
    ..oracle::JOHTULL_WURM
};
