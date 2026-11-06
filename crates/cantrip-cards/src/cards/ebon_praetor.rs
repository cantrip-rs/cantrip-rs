use cantrip_api::*;
use cantrip_mechanics::{targets, triggers};
use cantrip_oracle as oracle;

/// "At the beginning of your upkeep, put a -2/-2 counter on this creature." (CR 608.2, 122.1).
fn put_2_2_counter_creature(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    g.add_counters_together(&[(item.object, CounterKind::Minus2Minus2, 1)]);
    Ok(())
}

/// "Sacrifice a creature: Remove a -2/-2 counter from this creature. If the sacrificed creature was a Thrull, put a +1/+0 counter on this creature. Activate only during your upkeep and only once each turn." (CR 608.2, 122.1).
fn remove_2_2_counter_creature(
    g: &mut dyn Game,
    item: &StackItem,
) -> Result<(), Suspend> {
    g.remove_counters(item.object, CounterKind::Minus2Minus2, 1);
    g.end_sentence();
    let thrull = g.cost_objects(item).iter().any(|&id| {
        g.chars(id).has_subtype(g.words(item.object).subtype(Subtype::Thrull))
    });
    if thrull {
        g.add_counters_together(&[(item.object, CounterKind::Plus1Plus0, 1)]);
        g.end_sentence();
    }
    Ok(())
}

/// "Sacrifice a creature: Remove a -2/-2 counter from this creature. If the sacrificed creature was a Thrull, put a +1/+0 counter on this creature. Activate only during your upkeep and only once each turn." (CR 602.5).
fn remove_2_2_counter_creature_if(q: &dyn Query, asker: &Asker) -> bool {
    Some(asker.controller)
        .is_some_and(|player: PlayerId| {
            q.state().turn.active == player && q.state().turn.step() == Step::Upkeep
        })
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            triggers::YOUR_UPKEEP,
            put_2_2_counter_creature,
            "At the beginning of your upkeep, put a -2/-2 counter on this creature.",
        ),
        ability(
                sacrifice(1, targets::filters::creature),
                remove_2_2_counter_creature,
                "Sacrifice a creature: Remove a -2/-2 counter from this creature. If the sacrificed creature was a Thrull, put a +1/+0 counter on this creature. Activate only during your upkeep and only once each turn.",
            )
            .once_each_turn()
            .condition(remove_2_2_counter_creature_if),
    ],
    ..oracle::EBON_PRAETOR
};
