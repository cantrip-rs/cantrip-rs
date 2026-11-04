use cantrip_api::*;
use cantrip_mechanics::{mana, triggers};
use cantrip_oracle as oracle;

/// "At the beginning of your upkeep, if this land is tapped, put a storage counter on it." (CR 603.4).
fn if_land_tapped_put_storage_if(q: &dyn Query, asker: &Asker) -> bool {
    Some(asker.source).is_some_and(|candidate: ObjectId| q.state().obj(candidate).tapped)
}

/// "At the beginning of your upkeep, if this land is tapped, put a storage counter on it." (CR 608.2, 122.1).
fn if_land_tapped_put_storage(
    g: &mut dyn Game,
    item: &StackItem,
) -> Result<(), Suspend> {
    let mut counters_1: Vec<(ObjectId, CounterKind, i32)> = Vec::new();
    for t in Some(item.object).into_iter().map(Target::Object).collect::<Vec<Target>>() {
        if let Target::Object(object) = t {
            counters_1.push((object, CounterKind::Storage, 1));
        }
    }
    g.add_counters_together(&counters_1);
    Ok(())
}

/// "{T}, Remove any number of storage counters from this land: Add {R} for each storage counter removed this way." (CR 605.1a, 106.3).
fn add_r_storage_counter_removed_mana(
    g: &mut dyn Game,
    asker: &Asker,
    _: Option<Color>,
) -> Result<(), Suspend> {
    let mana_2_0 = {
        let q: &dyn Query = g;
        i64::from(q.mana_ability_x().unwrap_or(0)) as u32
    };
    g.add_mana(asker.controller, Mana::R, mana_2_0);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
                triggers::YOUR_UPKEEP,
                if_land_tapped_put_storage,
                "At the beginning of your upkeep, if this land is tapped, put a storage counter on it.",
            )
            .condition(if_land_tapped_put_storage_if),
        mana_ability(
                TAP
                    .and(Cost {
                        variable: Some(
                            VariableCost::RemoveAnyNumberOfCounters(CounterKind::Storage),
                        ),
                        ..Cost::NONE
                    }),
                add_by(add_r_storage_counter_removed_mana),
                "{T}, Remove any number of storage counters from this land: Add {R} for each storage counter removed this way.",
            )
            .x_is_mana_amount(),
    ],
    enters: Some(mana::enters_tapped),
    may_not_untap: true,
    ..oracle::DWARVEN_HOLD
};
