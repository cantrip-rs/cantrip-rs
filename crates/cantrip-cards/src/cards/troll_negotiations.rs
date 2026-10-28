use cantrip_api::*;
use cantrip_mechanics::fight::targets_fight;
use cantrip_mechanics::targets;
use cantrip_oracle as oracle;

/// "Put two +1/+1 counters on target creature you control. Then it fights target creature an opponent controls." (CR 122.1,
/// 701.14, 608.2b-c): each target is judged as the spell resolves. With the first gone nothing is put on and nothing fights; with
/// only the second gone the counters are still put on and no fight happens. The counters come first, so the fight is with the
/// power they give (CR 608.2c).
fn counters_then_fight(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if let Some(creature) = targets::object(g, item, 0) {
        g.add_counters(creature, CounterKind::PlusOne, 2);
    }
    g.end_sentence();
    targets_fight(g, item, 0, 1);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[targets::CREATURE_YOU_CONTROL, targets::CREATURE_AN_OPPONENT_CONTROLS], counters_then_fight),
    ..oracle::TROLL_NEGOTIATIONS
};
