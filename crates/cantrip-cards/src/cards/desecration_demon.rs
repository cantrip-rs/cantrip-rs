use cantrip_api::*;
use cantrip_mechanics::{effects, select, targets, triggers};
use cantrip_oracle as oracle;

/// "Any opponent may sacrifice a creature of their choice. If a player does, tap this creature and put a +1/+1 counter on it"
/// (CR 701.21a, 608.2c-d): each opponent in turn order (CR 101.4) chooses as the ability resolves, even after another has
/// sacrificed (the rulings), and the creature is tapped and gets one counter if any of them did - one, however many creatures were
/// sacrificed. A Demon that has left the battlefield is not tapped and gets nothing.
fn opponents_may_sacrifice_to_tap_and_grow(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let mut sacrificed = false;
    for opponent in g.state().apnap().filter(|&p| p != item.controller).collect::<Vec<_>>() {
        sacrificed |= !effects::may_sacrifice(g, opponent, item.object, select::creature_permanents(select::up_to(1)))?.is_empty();
    }
    g.end_sentence();
    if sacrificed && targets::permanent(g, item.object) {
        g.tap(item.object);
        g.add_counters(item.object, CounterKind::PlusOne, 1);
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        triggers::EACH_PLAYERS_BEGINNING_OF_COMBAT,
        opponents_may_sacrifice_to_tap_and_grow,
        "At the beginning of each combat, any opponent may sacrifice a creature of their choice. If a player does, tap this creature and put a +1/+1 counter on it.",
    )],
    ..oracle::DESECRATION_DEMON
};
