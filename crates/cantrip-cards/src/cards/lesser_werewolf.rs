use cantrip_api::*;
use cantrip_oracle as oracle;

/// "{B}: If this creature's power is 1 or more, it gets -1/-0 until end of turn and put a -0/-1 counter on target creature blocking or blocked by this creature. Activate only during the declare blockers step." (CR 115.1).
fn if_creatures_power_1_more_target_0(q: &dyn Query, asker: &Asker, t: Target) -> bool {
    matches!(
        t, Target::Object(id) if cantrip_mechanics::targets::permanent(q, id) && ((q
        .chars(id).is(CardType::Creature) && (Some(asker.source).is_some_and(| attacker |
        q.creatures_blocking(attacker).contains(& id)) || Some(asker.source)
        .is_some_and(| blocker | q.creatures_blocked_by(blocker).contains(& id)))))
    )
}

/// "{B}: If this creature's power is 1 or more, it gets -1/-0 until end of turn and put a -0/-1 counter on target creature blocking or blocked by this creature. Activate only during the declare blockers step." (CR 608.2, 611.2a, 122.1).
fn if_creatures_power_1_more(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let target = cantrip_mechanics::targets::object(g, item, 0)
        .filter(|_| g.chars(item.object).power.is_some_and(|power| power >= 1));
    if let Some(target) = target {
        instruction(g, |g| {
            g.add_effect(item.object, item.object, EffectKind::Pump(-1, 0), Duration::EndOfTurn);
            g.add_counters(target, CounterKind::Plus0Minus1, 1);
        });
    }
    Ok(())
}

/// "{B}: If this creature's power is 1 or more, it gets -1/-0 until end of turn and put a -0/-1 counter on target creature blocking or blocked by this creature. Activate only during the declare blockers step." (CR 602.5).
fn if_creatures_power_1_more_if(q: &dyn Query, _: &Asker) -> bool {
    q.state().turn.step() == Step::DeclareBlockers
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        ability(
                cost("{B}"),
                if_creatures_power_1_more,
                "{B}: If this creature's power is 1 or more, it gets -1/-0 until end of turn and put a -0/-1 counter on target creature blocking or blocked by this creature. Activate only during the declare blockers step.",
            )
            .targets(&[target(if_creatures_power_1_more_target_0)])
            .condition(if_creatures_power_1_more_if),
    ],
    ..oracle::LESSER_WEREWOLF
};
