use cantrip_api::*;
use cantrip_mechanics::{fight, history, targets};
use cantrip_oracle as oracle;

/// "Choose target creature you control and target creature you don't control. If the creature you control entered this turn, put a
/// +1/+1 counter on it. Then those creatures fight each other." Each target is judged as the spell resolves (CR 608.2b): with the
/// creature you control gone nothing is put on and nothing fights; with only the other gone the counter is still put on and nothing
/// fights (CR 701.14b). The counter comes first, so the fight is with the power it gives (CR 608.2c). Whether the creature entered this
/// turn is the object it is now (CR 400.7).
fn effect(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if targets::object(g, item, 0).is_some_and(|mine| history::entered_this_turn(g.history(), mine)) {
        return fight::plus_one_counters_on_first_target_then_it_fights_second::<1>(g, item);
    }
    fight::targets_fight(g, item, 0, 1);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[targets::CREATURE_YOU_CONTROL, targets::CREATURE_YOU_DONT_CONTROL], effect),
    ..oracle::MALAMET_BATTLE_GLYPH
};
