use cantrip_api::*;
use cantrip_mechanics::{statics, triggers};
use cantrip_oracle as oracle;

/// "At the beginning of your upkeep, put a time counter on this artifact." (CR 608.2, 122.1).
fn put_time_counter_artifact(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    g.add_counters(item.object, CounterKind::Time, 1);
    Ok(())
}

/// "All creatures get +1/+0 for each time counter on this artifact." (CR 604.1, 613.4c).
fn boost(q: &dyn LayerQuery, source: ObjectId, chars: &mut Chars) {
    let counters = q.state().obj(source).counters(CounterKind::Time);
    chars.power = chars.power.map(|power| power + counters);
}

/// "{3}: Remove a time counter from this artifact. Any player may activate this ability but only during any upkeep step." (CR 608.2).
fn remove_time_counter_artifact_any(
    g: &mut dyn Game,
    item: &StackItem,
) -> Result<(), Suspend> {
    g.remove_counters(item.object, CounterKind::Time, 1);
    Ok(())
}

/// "{3}: Remove a time counter from this artifact. Any player may activate this ability but only during any upkeep step." (CR 602.5).
fn remove_time_counter_artifact_any_if(q: &dyn Query, _: &Asker) -> bool {
    q.state().turn.step() == Step::Upkeep
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            triggers::YOUR_UPKEEP,
            put_time_counter_artifact,
            "At the beginning of your upkeep, put a time counter on this artifact.",
        ),
        ability(
                cost("{3}"),
                remove_time_counter_artifact_any,
                "{3}: Remove a time counter from this artifact. Any player may activate this ability but only during any upkeep step.",
            )
            .condition(remove_time_counter_artifact_any_if)
            .any_player_may_activate(),
    ],
    statics: &[
        static_ability(
            Layer::PtModify,
            statics::all_creatures,
            boost,
            "All creatures get +1/+0 for each time counter on this artifact.",
        ),
    ],
    ..oracle::INFINITE_HOURGLASS
};
