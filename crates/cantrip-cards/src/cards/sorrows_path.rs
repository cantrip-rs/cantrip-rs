use cantrip_api::*;
use cantrip_mechanics::{combat, targets, triggers};
use cantrip_oracle as oracle;

/// "{T}: Choose two target blocking creatures controlled by the same opponent. If each of those creatures could block all creatures that the other is blocking, remove both of them from combat. Each one then blocks all creatures the other was blocking." (CR 115.1).
fn choose_two_target_blocking_creatures_target_0(
    q: &dyn Query,
    asker: &Asker,
    t: Target,
) -> bool {
    matches!(
        t, Target::Object(id) if cantrip_mechanics::targets::permanent(q, id) && ((q
        .is_blocking(id) && q.chars(id).is(CardType::Creature) && q.state()
        .opponents(asker.controller).collect:: < Vec < PlayerId > > ().contains(& q
        .controller(id))))
    )
}

/// "{T}: Choose two target blocking creatures controlled by the same opponent. If each of those creatures could block all creatures that the other is blocking, remove both of them from combat. Each one then blocks all creatures the other was blocking." (CR 608.2).
fn choose_two_target_blocking_creatures(
    g: &mut dyn Game,
    item: &StackItem,
) -> Result<(), Suspend> {
    let first = targets::object(g, item, 0);
    let second = targets::object(g, item, 1);
    combat::swap_blocks(g, first, second, false);
    Ok(())
}

/// "Whenever this land becomes tapped, it deals 2 damage to you and each creature you control." (CR 608.2, 120.3).
fn deals_2_damage_creature_control(
    g: &mut dyn Game,
    item: &StackItem,
) -> Result<(), Suspend> {
    let recipients_1 = {
        let q: &dyn Query = g;
        {
            let mut all: Vec<Target> = Vec::new();
            for t in Some(item.controller)
                .into_iter()
                .map(Target::Player)
                .collect::<Vec<Target>>()
            {
                if !all.contains(&t) {
                    all.push(t);
                }
            }
            for t in q
                .state()
                .battlefield()
                .iter()
                .copied()
                .filter(|&id| {
                    q.chars(id).is(CardType::Creature)
                        && item.controller == q.controller(id)
                })
                .collect::<Vec<ObjectId>>()
                .into_iter()
                .map(Target::Object)
                .collect::<Vec<Target>>()
            {
                if !all.contains(&t) {
                    all.push(t);
                }
            }
            all
        }
    };
    let batch: Vec<(ObjectId, Target, u32)> = recipients_1
        .iter()
        .map(|t| (item.object, *t, 2u32))
        .collect();
    g.damage_together(&batch);
    Ok(())
}

/// "{T}: Choose two target blocking creatures controlled by the same opponent. If each of those creatures could block all creatures that the other is blocking, remove both of them from combat. Each one then blocks all creatures the other was blocking." (CR 608.2).
fn common_by(
    q: &dyn Query,
    choice: &Choice,
    key: fn(&dyn Query, ObjectId) -> PlayerId,
) -> Verdict {
    let first = choice.selected.first().map(|&c| key(q, c));
    Verdict {
        complete: choice.selected.iter().all(|&c| Some(key(q, c)) == first),
        addable: Bits::of(
            choice.candidates.len(),
            |i| first.map_or(true, |k| key(q, choice.candidates[i]) == k),
        ),
    }
}

/// "{T}: Choose two target blocking creatures controlled by the same opponent. If each of those creatures could block all creatures that the other is blocking, remove both of them from combat. Each one then blocks all creatures the other was blocking." (CR 608.2).
fn largest_by(
    q: &dyn Query,
    choice: &Choice,
    key: fn(&dyn Query, ObjectId) -> PlayerId,
) -> Vec<ObjectId> {
    choice
        .candidates
        .iter()
        .map(|&a| {
            choice
                .candidates
                .iter()
                .copied()
                .filter(|&b| key(q, b) == key(q, a))
                .collect::<Vec<_>>()
        })
        .max_by_key(Vec::len)
        .unwrap_or_default()
}

/// "{T}: Choose two target blocking creatures controlled by the same opponent. If each of those creatures could block all creatures that the other is blocking, remove both of them from combat. Each one then blocks all creatures the other was blocking." (CR 608.2).
fn controller_of(q: &dyn Query, id: ObjectId) -> PlayerId {
    q.controller(id)
}

/// "{T}: Choose two target blocking creatures controlled by the same opponent. If each of those creatures could block all creatures that the other is blocking, remove both of them from combat. Each one then blocks all creatures the other was blocking." (CR 608.2).
fn common_controller(q: &dyn Query, choice: &Choice, _: u32) -> Verdict {
    common_by(q, choice, controller_of)
}

/// "{T}: Choose two target blocking creatures controlled by the same opponent. If each of those creatures could block all creatures that the other is blocking, remove both of them from combat. Each one then blocks all creatures the other was blocking." (CR 608.2).
fn largest_common_controller(q: &dyn Query, choice: &Choice, _: u32) -> Vec<ObjectId> {
    largest_by(q, choice, controller_of)
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        ability(
                TAP,
                choose_two_target_blocking_creatures,
                "{T}: Choose two target blocking creatures controlled by the same opponent. If each of those creatures could block all creatures that the other is blocking, remove both of them from combat. Each one then blocks all creatures the other was blocking.",
            )
            .targets(
                &[
                    target(choose_two_target_blocking_creatures_target_0)
                        .between(2, 2)
                        .together(
                            cantrip_api::select::set_part(
                                &(common_controller as SelectFn),
                                &(largest_common_controller
                                    as cantrip_api::select::LargestFn),
                                0,
                                "with the same controller",
                            ),
                        ),
                ],
            ),
        triggered(
            triggers::THIS_BECOMES_TAPPED,
            deals_2_damage_creature_control,
            "Whenever this land becomes tapped, it deals 2 damage to you and each creature you control.",
        ),
    ],
    ..oracle::SORROWS_PATH
};
