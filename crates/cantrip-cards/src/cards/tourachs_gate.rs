use cantrip_api::*;
use cantrip_mechanics::{targets, triggers};
use cantrip_oracle as oracle;

/// "Sacrifice a Thrull: Put three time counters on this Aura." (CR 608.2, 122.1).
fn put_three_time_counters_aura(
    g: &mut dyn Game,
    item: &StackItem,
) -> Result<(), Suspend> {
    let mut counters_0: Vec<(ObjectId, CounterKind, i32)> = Vec::new();
    for t in Some(item.object).into_iter().map(Target::Object).collect::<Vec<Target>>() {
        if let Target::Object(object) = t {
            counters_0.push((object, CounterKind::Time, 3i32));
        }
    }
    g.add_counters_together(&counters_0);
    Ok(())
}

/// "Sacrifice a Thrull: Put three time counters on this Aura." (CR 118.1).
fn put_three_time_counters_aura_2(q: &dyn Query, asker: &Asker, id: ObjectId) -> bool {
    q.chars(id).has_subtype(q.words(asker.source).subtype(Subtype::Thrull))
}

/// "At the beginning of your upkeep, remove a time counter from this Aura. If there are no time counters on this Aura, sacrifice it." (CR 608.2, 701.21a).
fn remove_time_counter_aura_if(
    g: &mut dyn Game,
    item: &StackItem,
) -> Result<(), Suspend> {
    let mut hoisted_5: Vec<ObjectId> = Vec::new();
    let mut hoisted_4: Vec<ObjectId> = Vec::new();
    let objects_2_0 = Some(item.object).into_iter().collect::<Vec<ObjectId>>();
    instruction(
        g,
        |g| {
            for object in objects_2_0 {
                let _ = g.remove_counters(object, CounterKind::Time, 1);
            }
        },
    );
    g.end_sentence();
    if {
        let q: &dyn Query = g;
        Some(item.object)
            .is_some_and(|candidate: ObjectId| {
                (i64::from(q.state().obj(candidate).counters(CounterKind::Time))) == 0
            })
    } {
        let objects_3 = Some(item.object).into_iter().collect::<Vec<ObjectId>>();
        let players_3 = Some(item.controller).into_iter().collect::<Vec<PlayerId>>();
        let mut picks_3: Vec<(PlayerId, ObjectId)> = Vec::new();
        for &object in &objects_3 {
            let controller = g.controller(object);
            if players_3.contains(&controller) && g.can_sacrifice(controller, object) {
                picks_3.push((controller, object));
            }
        }
        let sacrificed_3 = g.sacrifice_each(&picks_3);
        let sacrificed_3 = sacrificed_3.into_objects();
        g.end_sentence();
        hoisted_4.extend(sacrificed_3.clone());
        hoisted_5
            .extend({
                let q: &dyn Query = g;
                objects_3
                    .iter()
                    .copied()
                    .filter(|&o| q.state().obj(o).became.is_some())
                    .collect::<Vec<ObjectId>>()
            });
    }
    Ok(())
}

/// "Tap enchanted land: Attacking creatures you control get +2/-1 until end of turn. Activate only if enchanted land is untapped." (CR 608.2, 611.2a).
fn attacking_creatures_control_get_2(
    g: &mut dyn Game,
    item: &StackItem,
) -> Result<(), Suspend> {
    let objects_6 = {
        let q: &dyn Query = g;
        q.state()
            .battlefield()
            .iter()
            .copied()
            .filter(|&id| {
                q.is_attacking(id)
                    && q.chars(id).is(CardType::Creature)
                    && item.controller == q.controller(id)
            })
            .collect::<Vec<ObjectId>>()
    };
    instruction(
        g,
        |g| {
            for &object in &objects_6 {
                g.add_effect(
                    item.object,
                    object,
                    EffectKind::Pump(2, -1),
                    Duration::EndOfTurn,
                );
            }
        },
    );
    Ok(())
}

/// "Tap enchanted land: Attacking creatures you control get +2/-1 until end of turn. Activate only if enchanted land is untapped." (CR 118.1).
fn attacking_creatures_control_get_2_2(
    q: &dyn Query,
    asker: &Asker,
    id: ObjectId,
) -> bool {
    Some(asker.source).and_then(|o: ObjectId| q.attached_to(o)) == Some(id)
}

/// "Tap enchanted land: Attacking creatures you control get +2/-1 until end of turn. Activate only if enchanted land is untapped." (CR 602.5).
fn attacking_creatures_control_get_2_if(q: &dyn Query, asker: &Asker) -> bool {
    Some(asker.source)
        .and_then(|o: ObjectId| q.attached_to(o))
        .is_some_and(|candidate: ObjectId| !(q.state().obj(candidate).tapped))
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        ability(
            sacrifice(1, put_three_time_counters_aura_2),
            put_three_time_counters_aura,
            "Sacrifice a Thrull: Put three time counters on this Aura.",
        ),
        triggered(
            triggers::YOUR_UPKEEP,
            remove_time_counter_aura_if,
            "At the beginning of your upkeep, remove a time counter from this Aura. If there are no time counters on this Aura, sacrifice it.",
        ),
        ability(
                tap_untapped(1, attacking_creatures_control_get_2_2),
                attacking_creatures_control_get_2,
                "Tap enchanted land: Attacking creatures you control get +2/-1 until end of turn. Activate only if enchanted land is untapped.",
            )
            .condition(attacking_creatures_control_get_2_if),
    ],
    enchant: Some(targets::LAND_YOU_CONTROL),
    ..oracle::TOURACHS_GATE
};
