use cantrip_api::*;
use cantrip_mechanics::{damage_target, targets, triggers};
use cantrip_oracle as oracle;

/// "At the beginning of your upkeep, put a pin counter on this artifact." (CR 608.2, 122.1).
fn put_pin_counter_artifact(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    g.add_counters(item.object, CounterKind::Pin, 1);
    Ok(())
}

/// "At the beginning of your end step, if this artifact is untapped, destroy this artifact and it deals damage to you equal to the number of pin counters on it." (CR 603.4).
fn if_artifact_untapped_destroy_artifact_if(q: &dyn Query, asker: &Asker) -> bool {
    !q.obj(asker.source).tapped
}

/// "At the beginning of your end step, if this artifact is untapped, destroy this artifact and it deals damage to you equal to the number of pin counters on it." (CR 608.2, 120.3, 701.8a).
fn if_artifact_untapped_destroy_artifact(
    g: &mut dyn Game,
    item: &StackItem,
) -> Result<(), Suspend> {
    g.destroy(item.object);
    g.end_sentence();
    let amount = g.obj(item.object).counters(CounterKind::Pin).max(0) as u32;
    g.damage_together(&[(item.object, Target::Player(item.controller), amount)]);
    Ok(())
}

/// "{X}{X}, {T}: This artifact deals damage equal to the number of pin counters on it to any target. X is the number of pin counters on this artifact." (CR 608.2, 120.3).
fn artifact_deals_damage_equal_number(
    g: &mut dyn Game,
    item: &StackItem,
) -> Result<(), Suspend> {
    let amount = g.obj(item.object).counters(CounterKind::Pin).max(0) as u32;
    damage_target(g, item, 0, amount);
    Ok(())
}

/// "{X}{X}, {T}: This artifact deals damage equal to the number of pin counters on it to any target. X is the number of pin counters on this artifact." (CR 608.2).
fn artifact_deals_damage_equal_number_x_is(
    q: &dyn Query,
    asker: &Asker,
    _: &[Target],
) -> u32 {
    q.obj(asker.source).counters(CounterKind::Pin).max(0) as u32
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            triggers::YOUR_UPKEEP,
            put_pin_counter_artifact,
            "At the beginning of your upkeep, put a pin counter on this artifact.",
        ),
        triggered(
                triggers::YOUR_END_STEP,
                if_artifact_untapped_destroy_artifact,
                "At the beginning of your end step, if this artifact is untapped, destroy this artifact and it deals damage to you equal to the number of pin counters on it.",
            )
            .condition(if_artifact_untapped_destroy_artifact_if),
        ability(
                cost("{X}{X}").and(TAP),
                artifact_deals_damage_equal_number,
                "{X}{X}, {T}: This artifact deals damage equal to the number of pin counters on it to any target. X is the number of pin counters on this artifact.",
            )
            .targets(&[targets::ANY])
            .x_is(artifact_deals_damage_equal_number_x_is),
    ],
    ..oracle::VOODOO_DOLL
};
