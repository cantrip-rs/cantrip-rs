use cantrip_api::*;
use cantrip_mechanics::triggers;
use cantrip_oracle as oracle;

/// "{4}: Remove a paralyzation counter from this creature." (CR 608.2).
fn granted_5(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let objects_6_0 = Some(item.object).into_iter().collect::<Vec<ObjectId>>();
    instruction(
        g,
        |g| {
            for object in objects_6_0 {
                let _ = g.remove_counters(object, CounterKind::Paralyzation, 1);
            }
        },
    );
    Ok(())
}

/// "At end of combat, put a paralyzation counter on each creature blocking or blocked by this creature and tap those creatures. Each of those creatures doesn't untap during its controller's untap step for as long as it has a paralyzation counter on it. Each of those creatures gains "{4}: Remove a paralyzation counter from this creature."" (CR 608.2, 611.2a, 122.1).
fn remove_paralyzation_counter_creature(
    g: &mut dyn Game,
    item: &StackItem,
) -> Result<(), Suspend> {
    let noted_0: Vec<ObjectId> = {
        let q: &dyn Query = g;
        q.state()
            .battlefield()
            .iter()
            .copied()
            .filter(|&id| {
                q.chars(id).is(CardType::Creature)
                    && (q
                        .state()
                        .blocks
                        .iter()
                        .any(|b| b.blocker == id && b.attacker == item.object)
                        || q
                            .state()
                            .blocks
                            .iter()
                            .any(|b| b.attacker == id && b.blocker == item.object))
            })
            .collect::<Vec<ObjectId>>()
    };
    let mut counters_1: Vec<(ObjectId, CounterKind, i32)> = Vec::new();
    for t in noted_0.clone().into_iter().map(Target::Object).collect::<Vec<Target>>() {
        if let Target::Object(object) = t {
            counters_1.push((object, CounterKind::Paralyzation, 1));
        }
    }
    g.add_counters_together(&counters_1);
    let objects_2 = noted_0.clone();
    g.tap_together(&objects_2);
    g.end_sentence();
    let objects_3 = {
        let q: &dyn Query = g;
        noted_0
            .clone()
            .into_iter()
            .filter(|&o| is_live(q, o))
            .collect::<Vec<ObjectId>>()
    };
    let kinds_3 = {
        let mut all = Vec::new();
        for &object in &objects_3 {
            all.push((object, EffectKind::DoesntUntap));
        }
        all
    };
    instruction(
        g,
        |g| {
            for &(object, kind) in &kinds_3 {
                g.add_effect(
                    item.object,
                    object,
                    kind,
                    Duration::WhileCounter(CounterKind::Paralyzation),
                );
            }
        },
    );
    g.end_sentence();
    let objects_4 = noted_0.clone();
    let kind_4_0_0 = {
        let q: &dyn Query = g;
        EffectKind::AddAbility(
            AbilityRef::delayed(
                {
                    let o = q.state().obj(item.object);
                    o.copied_card().unwrap_or(o.card)
                },
                0,
            ),
        )
    };
    instruction(
        g,
        |g| {
            for &object in &objects_4 {
                g.add_effect(item.object, object, kind_4_0_0, Duration::Indefinite);
            }
        },
    );
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            triggers::END_OF_COMBAT,
            remove_paralyzation_counter_creature,
            "At end of combat, put a paralyzation counter on each creature blocking or blocked by this creature and tap those creatures. Each of those creatures doesn't untap during its controller's untap step for as long as it has a paralyzation counter on it. Each of those creatures gains \"{4}: Remove a paralyzation counter from this creature.\"",
        ),
    ],
    delayed: &[
        ability(
            cost("{4}"),
            granted_5,
            "{4}: Remove a paralyzation counter from this creature.",
        ),
    ],
    ..oracle::DREAD_WIGHT
};
