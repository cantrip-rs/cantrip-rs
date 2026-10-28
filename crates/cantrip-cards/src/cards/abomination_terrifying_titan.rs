use cantrip_api::*;
use cantrip_mechanics::{power_up, targets};
use cantrip_oracle as oracle;

/// "Put a +1/+1 counter on Abomination. He fights up to one target creature an opponent controls." The counter goes on
/// whether or not a target was chosen; he fights only if the target is still legal as it resolves (CR 608.2b, 701.14).
fn counter_and_fight(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    g.add_counters(item.object, CounterKind::PlusOne, 1);
    let Some(other) = targets::object(g, item, 0) else { return Ok(()) };
    g.fight(item.object, other);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[power_up::ability(
        cost("{5}{R/G}{R/G}"),
        counter_and_fight,
        "Power-up \u{2014} {5}{R/G}{R/G}: Put a +1/+1 counter on Abomination. He fights up to one target creature an opponent controls. (Activate each power-up ability only once. Reduce the cost by his mana cost if he entered this turn.)",
    )
    .targets(&[targets::CREATURE_AN_OPPONENT_CONTROLS.optional()])],
    ..oracle::ABOMINATION_TERRIFYING_TITAN
};
