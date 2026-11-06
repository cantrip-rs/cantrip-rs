use cantrip_api::*;
use cantrip_oracle as oracle;

/// "{2}, {T}: Put a charge counter on this artifact." (CR 608.2, 122.1).
fn put_charge_counter_artifact(
    g: &mut dyn Game,
    item: &StackItem,
) -> Result<(), Suspend> {
    let mut counters_0: Vec<(ObjectId, CounterKind, i32)> = Vec::new();
    for t in Some(item.object).into_iter().map(Target::Object).collect::<Vec<Target>>() {
        if let Target::Object(object) = t {
            counters_0.push((object, CounterKind::Charge, 1));
        }
    }
    g.add_counters_together(&counters_0);
    Ok(())
}

/// "{T}, Remove any number of charge counters from this artifact: Add {W}, then add an additional {W} for each charge counter removed this way." (CR 605.1a, 106.3).
fn add_w_add_additional_w_mana(
    g: &mut dyn Game,
    asker: &Asker,
    _: Option<Color>,
) -> Result<(), Suspend> {
    g.add_mana(asker.controller, Mana::W, 1);
    let mana_3_0 = {
        let q: &dyn Query = g;
        i64::from(q.mana_ability_x().unwrap_or(0)) as u32
    };
    g.add_mana(asker.controller, Mana::W, mana_3_0);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        ability(
            cost("{2}").and(TAP),
            put_charge_counter_artifact,
            "{2}, {T}: Put a charge counter on this artifact.",
        ),
        mana_ability(
                TAP
                    .and(Cost {
                        variable: Some(
                            VariableCost::RemoveAnyNumberOfCounters(CounterKind::Charge),
                        ),
                        ..Cost::NONE
                    }),
                Adds {
                    pool: ManaPool([1, 0, 0, 0, 0, 0]),
                    ..add_by(add_w_add_additional_w_mana)
                },
                "{T}, Remove any number of charge counters from this artifact: Add {W}, then add an additional {W} for each charge counter removed this way.",
            )
            .x_is_mana_amount(),
    ],
    ..oracle::WHITE_MANA_BATTERY
};
