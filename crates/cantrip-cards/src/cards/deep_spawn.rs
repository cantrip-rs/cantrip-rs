use cantrip_api::*;
use cantrip_mechanics::{effects, triggers};
use cantrip_oracle as oracle;

const MILL_TWO: Question = Question::new(
    "Mill", "mill two cards?", "If you don't, sacrifice this creature", "Mill two", "Don't mill",
);

/// "At the beginning of your upkeep, sacrifice this creature unless you mill two cards." (CR 608.2, 701.21a, 701.17a).
fn sacrifice_creature_unless_mill_two(
    g: &mut dyn Game,
    item: &StackItem,
) -> Result<(), Suspend> {
    let paid = g.state().zone(Zone::library(item.controller)).len() >= 2
        && effects::ask_may(g, item.controller, item.object, Purpose::Ask(&MILL_TWO))?;
    if paid {
        g.mill_together(&[(item.controller, 2)]);
    }
    g.end_sentence();
    if !paid {
        g.sacrifice(item.controller, &[item.object]);
        g.end_sentence();
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            triggers::YOUR_UPKEEP,
            sacrifice_creature_unless_mill_two,
            "At the beginning of your upkeep, sacrifice this creature unless you mill two cards.",
        ),
        ability(
            cost("{U}"),
            effects::this_gains_shroud_doesnt_untap_next_untap_step_then_taps,
            "{U}: This creature gains shroud until end of turn and doesn't untap during your next untap step. Tap this creature.",
        ),
    ],
    ..oracle::DEEP_SPAWN
};
