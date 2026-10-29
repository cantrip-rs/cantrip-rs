use cantrip_api::*;
use cantrip_mechanics::{effects, triggers};
use cantrip_oracle as oracle;

/// "Captain Burnham deals 1 damage to each opponent. If this is the second time this ability has resolved this turn,
/// put a +1/+1 counter on Captain Burnham." Earlier resolutions of this ability this turn are read from the event history.
/// Each resolution follows its instructions in order (CR 608.2c): the damage happens every time, and the counter is added
/// only on the second.
fn landfall(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let second_resolution = g.history().resolutions(item.object, item.ability) == 1;
    effects::damage_each_opponent(g, item, 1);
    if second_resolution {
        g.add_counters(item.object, CounterKind::PlusOne, 1);
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        triggers::LANDFALL,
        landfall,
        "Landfall — Whenever a land you control enters, Captain Burnham deals 1 damage to each opponent. If this is the second time this ability has resolved this turn, put a +1/+1 counter on Captain Burnham.",
    )],
    ..oracle::CAPTAIN_MICHAEL_BURNHAM
};
