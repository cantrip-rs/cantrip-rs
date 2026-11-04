use cantrip_api::*;
use cantrip_mechanics::flashback;
use cantrip_oracle as oracle;

/// "Attacking creatures get +2/+0 until end of turn" (CR 611.2c): the creatures attacking as the spell resolves, whoever controls
/// them, and none that attack or are put onto the battlefield attacking later.
fn effect(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let attackers: Vec<ObjectId> = g.state().battlefield().iter().copied().filter(|&id| g.is_attacking(id)).collect();
    instruction(g, |g| {
        for id in attackers {
            cantrip_mechanics::pump(g, item.object, id, 2, 0);
        }
    });
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[], effect),
    ways: &[flashback::way(cost("{2}{W}{W}"), "Flashback {2}{W}{W}")],
    ..oracle::AURONS_INSPIRATION
};
