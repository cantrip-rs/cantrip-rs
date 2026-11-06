use cantrip_api::*;
use cantrip_mechanics::triggers;
use cantrip_oracle as oracle;

/// "This land doesn't untap during your untap step if it has a depletion counter on it. At the beginning of your upkeep, remove a depletion counter from this land. {T}: Add {B} or {R}. Put a depletion counter on this land." (CR 604.1).
fn add_b_r_put_depletion_rule(q: &dyn Query, asker: &Asker, id: ObjectId) -> bool {
    (Some(asker.source)
        .is_some_and(|candidate: ObjectId| {
            (i64::from(q.state().obj(candidate).counters(CounterKind::Depletion))) > 0
        })) && id == asker.source
}

/// "At the beginning of your upkeep, remove a depletion counter from this land." (CR 608.2).
fn remove_depletion_counter_land(
    g: &mut dyn Game,
    item: &StackItem,
) -> Result<(), Suspend> {
    let objects_0_0 = Some(item.object).into_iter().collect::<Vec<ObjectId>>();
    instruction(
        g,
        |g| {
            for object in objects_0_0 {
                let _ = g.remove_counters(object, CounterKind::Depletion, 1);
            }
        },
    );
    Ok(())
}

/// "{T}: Add {B} or {R}. Put a depletion counter on this land." (CR 605.3a, 122.1, 106.3).
fn add_b_r_put_depletion_mana(
    g: &mut dyn Game,
    asker: &Asker,
    chosen: Option<Color>,
) -> Result<(), Suspend> {
    {
        let Some(color) = chosen else { return Ok(()) };
        g.add_mana(asker.controller, color.mana(), 1);
    }
    let mut counters_2: Vec<(ObjectId, CounterKind, i32)> = Vec::new();
    for t in Some(asker.source).into_iter().map(Target::Object).collect::<Vec<Target>>()
    {
        if let Target::Object(object) = t {
            counters_2.push((object, CounterKind::Depletion, 1));
        }
    }
    g.add_counters_together(&counters_2);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            triggers::YOUR_UPKEEP,
            remove_depletion_counter_land,
            "At the beginning of your upkeep, remove a depletion counter from this land.",
        ),
        mana_ability(
            TAP,
            add_by(add_b_r_put_depletion_mana)
                .choose_color()
                .colors(ColorSet(12))
                .yields_any(1),
            "{T}: Add {B} or {R}. Put a depletion counter on this land.",
        ),
    ],
    untap_bans: &[add_b_r_put_depletion_rule],
    ..oracle::LAVA_TUBES
};
