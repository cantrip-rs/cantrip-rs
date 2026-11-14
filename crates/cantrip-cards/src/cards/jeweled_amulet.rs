use cantrip_api::*;
use cantrip_mechanics::notes;
use cantrip_oracle as oracle;

/// "{1}, {T}: Put a charge counter on this artifact. Note the type of mana spent to pay this activation cost. Activate only if there are no charge counters on this artifact." (CR 608.2, 122.1).
fn put_charge_counter_artifact_note(
    g: &mut dyn Game,
    item: &StackItem,
) -> Result<(), Suspend> {
    let mut counters_0: Vec<(ObjectId, CounterKind, i32)> = Vec::new();
    for t in Some(item.object).into_iter().map(Target::Object).collect::<Vec<Target>>() {
        if let Target::Object(object) = t {
            counters_0.push((object, CounterKind::Charge, 1i32));
        }
    }
    g.add_counters_together(&counters_0);
    g.end_sentence();
    notes::note_mana_spent(g, item, false);
    Ok(())
}

/// "{1}, {T}: Put a charge counter on this artifact. Note the type of mana spent to pay this activation cost. Activate only if there are no charge counters on this artifact." (CR 602.5).
fn put_charge_counter_artifact_note_if(q: &dyn Query, asker: &Asker) -> bool {
    Some(asker.source)
        .is_some_and(|candidate: ObjectId| {
            (i64::from(q.state().obj(candidate).counters(CounterKind::Charge))) == 0
        })
}

/// "{T}, Remove a charge counter from this artifact: Add one mana of this artifact's last noted type." (CR 605.1a).
fn add_one_mana_artifacts_last_mana(
    g: &mut dyn Game,
    asker: &Asker,
    _: Option<Color>,
) -> Result<(), Suspend> {
    notes::add_last_mana(g, asker, asker.controller, false, None)
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        ability(
                cost("{1}").and(TAP),
                put_charge_counter_artifact_note,
                "{1}, {T}: Put a charge counter on this artifact. Note the type of mana spent to pay this activation cost. Activate only if there are no charge counters on this artifact.",
            )
            .tracks_mana_spent()
            .condition(put_charge_counter_artifact_note_if),
        mana_ability(
            TAP.and(remove_counters(CounterKind::Charge, 1)),
            add_by(add_one_mana_artifacts_last_mana),
            "{T}, Remove a charge counter from this artifact: Add one mana of this artifact's last noted type.",
        ),
    ],
    ..oracle::JEWELED_AMULET
};
