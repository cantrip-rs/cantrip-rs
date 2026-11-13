use cantrip_api::*;
use cantrip_mechanics::triggers::CREATURE_ATTACKS_YOU;
use cantrip_mechanics::{effects, pump};
use cantrip_oracle as oracle;

/// "Those creatures get -1/-0 until end of turn" (CR 611.2c): the creatures of the batch that are still on the battlefield as
/// the ability resolves; a creature that has left, or become a new object, is not affected.
fn attackers_get_minus_one_power(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let creatures = g.event_objects(item).to_vec();
    instruction(g, |g| {
        for creature in creatures {
            pump(g, item.object, creature, -1, 0);
        }
    });
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(CREATURE_ATTACKS_YOU, attackers_get_minus_one_power, "Whenever one or more creatures attack you, those creatures get -1/-0 until end of turn.")
            .once_per_batch(),
        ability(
            cost("{5}{U}{U}"),
            effects::three_plus_one_counters_on_this,
            "Exhaust \u{2014} {5}{U}{U}: Put three +1/+1 counters on this creature. (Activate each exhaust ability only once.)",
        )
        .exhaust(),
    ],
    ..oracle::SABOTAGE_STRATEGIST
};
