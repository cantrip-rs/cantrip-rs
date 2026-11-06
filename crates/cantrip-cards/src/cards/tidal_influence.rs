use cantrip_api::*;
use cantrip_mechanics::{statics, triggers};
use cantrip_oracle as oracle;

/// "Cast this spell only if no permanents named Tidal Influence are on the battlefield." (CR 601.2e).
fn cast_spell_only_if_no_if(q: &dyn Query, _: &Asker) -> bool {
    (q
        .state()
        .battlefield()
        .iter()
        .copied()
        .filter(|&id| q.chars(id).name == "Tidal Influence")
        .collect::<Vec<ObjectId>>()
        .into_iter()
        .filter(|&id| {
            q.state().obj(id).became.is_none()
                && q.state().obj(id).zone.kind() == ZoneKind::Battlefield
        })
        .collect::<Vec<ObjectId>>()
        .len() as i64) == 0
}

/// "This enchantment enters with a tide counter on it." (CR 614.1c, 122.6).
fn cast_spell_only_if_no_enters(
    _: &mut dyn Game,
    entering: &mut Entering,
) -> Result<(), Suspend> {
    entering.counters[CounterKind::Tide as usize] = entering
        .counters[CounterKind::Tide as usize]
        .saturating_add(1);
    Ok(())
}

/// "At the beginning of your upkeep, put a tide counter on this enchantment." (CR 603.2, 122.1).
fn put_tide_counter_enchantment(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    g.add_counters(item.object, CounterKind::Tide, 1);
    Ok(())
}

/// "As long as there is exactly one tide counter on this enchantment, all blue creatures get -2/-0." (CR 604.1).
fn as_long_as_there_exactly_affects(
    q: &dyn LayerQuery,
    source: ObjectId,
    _: ObjectId,
    so_far: &Chars,
) -> bool {
    let asker = &Asker::new(source, q.controller(source));
    (Some(asker.source)
        .is_some_and(|candidate: ObjectId| {
            (i64::from(q.state().obj(candidate).counters(CounterKind::Tide))) == 1
        }))
        && (((*so_far).colors.has(q.words(asker.source).color(Color::Blue))
            && (*so_far).is(CardType::Creature)))
}

/// "As long as there are exactly three tide counters on this enchantment, all blue creatures get +2/+0." (CR 604.1).
fn as_long_as_there_exactly_affects_2(
    q: &dyn LayerQuery,
    source: ObjectId,
    _: ObjectId,
    so_far: &Chars,
) -> bool {
    let asker = &Asker::new(source, q.controller(source));
    (Some(asker.source)
        .is_some_and(|candidate: ObjectId| {
            (i64::from(q.state().obj(candidate).counters(CounterKind::Tide))) == 3
        }))
        && (((*so_far).colors.has(q.words(asker.source).color(Color::Blue))
            && (*so_far).is(CardType::Creature)))
}

/// "Whenever there are four or more tide counters on this enchantment" (CR 603.8).
fn cast_spell_only_if_no_state(q: &dyn Query, asker: &Asker) -> bool {
    Some(asker.source)
        .is_some_and(|candidate: ObjectId| {
            (i64::from(q.state().obj(candidate).counters(CounterKind::Tide))) >= 4
        })
}

/// "Remove all tide counters from it." (CR 603.8).
fn cast_spell_only_if_no(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
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
            put_tide_counter_enchantment,
            "At the beginning of your upkeep, put a tide counter on this enchantment.",
        ),
        state_triggered(
            cast_spell_only_if_no_state,
            cast_spell_only_if_no,
            "Whenever there are four or more tide counters on this enchantment, remove all tide counters from it.",
        ),
    ],
    statics: &[
        static_ability(
            Layer::PtModify,
            as_long_as_there_exactly_affects,
            statics::plus_pt_saturating::<-2, 0>,
            "As long as there is exactly one tide counter on this enchantment, all blue creatures get -2/-0.",
        ),
        static_ability(
            Layer::PtModify,
            as_long_as_there_exactly_affects_2,
            statics::plus_pt_saturating::<2, 0>,
            "As long as there are exactly three tide counters on this enchantment, all blue creatures get +2/+0.",
        ),
    ],
    enters: Some(cast_spell_only_if_no_enters),
    cast_only_if: Some(cast_spell_only_if_no_if),
    ..oracle::TIDAL_INFLUENCE
};
