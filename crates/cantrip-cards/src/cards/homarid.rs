use cantrip_api::*;
use cantrip_mechanics::{effects, statics, triggers};
use cantrip_oracle as oracle;

/// "At the beginning of your upkeep, put a tide counter on this creature." (CR 608.2, 122.1).
fn put_tide_counter_creature(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    g.add_counters(item.object, CounterKind::Tide, 1);
    Ok(())
}

/// "As long as there is exactly one tide counter on this creature, it gets -1/-1." (CR 604.1).
fn as_long_as_there_exactly_affects(
    q: &dyn LayerQuery,
    source: ObjectId,
    affected: ObjectId,
    _: &Chars,
) -> bool {
    let asker = &Asker::new(source, q.controller(source));
    (Some(asker.source)
        .is_some_and(|candidate: ObjectId| {
            (i64::from(q.state().obj(candidate).counters(CounterKind::Tide))) == 1
        })) && (source == affected)
}

/// "As long as there are exactly three tide counters on this creature, it gets +1/+1." (CR 604.1).
fn as_long_as_there_exactly_affects_2(
    q: &dyn LayerQuery,
    source: ObjectId,
    affected: ObjectId,
    _: &Chars,
) -> bool {
    let asker = &Asker::new(source, q.controller(source));
    (Some(asker.source)
        .is_some_and(|candidate: ObjectId| {
            (i64::from(q.state().obj(candidate).counters(CounterKind::Tide))) == 3
        })) && (source == affected)
}

/// "This creature enters with a tide counter on it. At the beginning of your upkeep, put a tide counter on this creature. As long as there is exactly one tide counter on this creature, it gets -1/-1. As long as there are exactly three tide counters on this creature, it gets +1/+1. Whenever there are four or more tide counters on this creature, remove all tide counters from it." (CR 608.2).
fn creature_enters_tide_counter_beginning_state(q: &dyn Query, asker: &Asker) -> bool {
    Some(asker.source)
        .is_some_and(|candidate: ObjectId| {
            (i64::from(q.state().obj(candidate).counters(CounterKind::Tide))) >= 4
        })
}

/// "This creature enters with a tide counter on it. At the beginning of your upkeep, put a tide counter on this creature. As long as there is exactly one tide counter on this creature, it gets -1/-1. As long as there are exactly three tide counters on this creature, it gets +1/+1. Whenever there are four or more tide counters on this creature, remove all tide counters from it." (CR 608.2).
fn creature_enters_tide_counter_beginning(
    g: &mut dyn Game,
    item: &StackItem,
) -> Result<(), Suspend> {
    let objects_2_0 = Some(item.object).into_iter().collect::<Vec<ObjectId>>();
    instruction(
        g,
        |g| {
            for object in objects_2_0 {
                let _ = g.remove_counters(object, CounterKind::Tide, i32::MAX);
            }
        },
    );
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            triggers::YOUR_UPKEEP,
            put_tide_counter_creature,
            "At the beginning of your upkeep, put a tide counter on this creature.",
        ),
        state_triggered(
            creature_enters_tide_counter_beginning_state,
            creature_enters_tide_counter_beginning,
            "Whenever there are four or more tide counters on this creature, remove all tide counters from it.",
        ),
    ],
    statics: &[
        static_ability(
            Layer::PtModify,
            as_long_as_there_exactly_affects,
            statics::plus_pt_saturating::<-1, -1>,
            "As long as there is exactly one tide counter on this creature, it gets -1/-1.",
        ),
        static_ability(
            Layer::PtModify,
            as_long_as_there_exactly_affects_2,
            statics::plus_pt_saturating::<1, 1>,
            "As long as there are exactly three tide counters on this creature, it gets +1/+1.",
        ),
    ],
    enters: Some(effects::enters_with_a_counter::<{ CounterKind::Tide as usize }>),
    ..oracle::HOMARID
};
