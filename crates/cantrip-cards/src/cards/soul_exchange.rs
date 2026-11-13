use cantrip_api::*;
use cantrip_mechanics::targets;
use cantrip_oracle as oracle;

/// A copy of a card is not a card, even before state-based actions remove it (CR 704.5e).
fn creature_card(q: &dyn Query, asker: &Asker, t: Target) -> bool {
    (targets::CREATURE_CARD_IN_YOUR_GRAVEYARD.filter)(q, asker, t)
        && matches!(t, Target::Object(id) if !q.obj(id).card_copy)
}

/// "As an additional cost to cast this spell, exile a creature you control. Return target creature card from your graveyard to the battlefield. Put a +2/+2 counter on that creature if the exiled creature was a Thrull." (CR 118.1).
fn as_additional_cost_cast_spell(q: &dyn Query, asker: &Asker, id: ObjectId) -> bool {
    q.chars(id).is(CardType::Creature) && asker.controller == q.controller(id)
}

/// "As an additional cost to cast this spell, exile a creature you control. Return target creature card from your graveyard to the battlefield. Put a +2/+2 counter on that creature if the exiled creature was a Thrull." (CR 608.2, 122.1).
fn as_additional_cost_cast_spell_2(
    g: &mut dyn Game,
    item: &StackItem,
) -> Result<(), Suspend> {
    let mut hoisted_3: Vec<ObjectId> = Vec::new();
    let t_0 = vec![g.state().target(item, 0)];
    let objects_1 = t_0
        .get(0)
        .copied()
        .and_then(|t: Target| t.object())
        .into_iter()
        .filter(|&o: &ObjectId| !g.state().obj(o).token)
        .collect::<Vec<ObjectId>>();
    let moved_1 = {
        let rows_1: Vec<(ObjectId, PlayerId, bool)> = objects_1
            .iter()
            .map(|&o| (o, item.controller, false))
            .collect();
        let came_1 = g.put_all_onto_battlefield_as(&rows_1)?;
        came_1
            .into_iter()
            .filter(|&o| g.state().obj(o).zone.is(ZoneKind::Battlefield))
            .collect::<Vec<ObjectId>>()
    };
    g.end_sentence();
    if {
        let q: &dyn Query = g;
        (q
            .history()
            .spell_cost_objects(
                item.object,
                CostOrigin::PrintedAdditional,
                0,
                CostPart::Permanents,
            )
            .collect::<Vec<ObjectId>>()
            .into_iter()
            .filter(|&id| {
                q.chars(id).is(CardType::Creature)
                    && q
                        .chars(id)
                        .has_subtype(q.words(item.object).subtype(Subtype::Thrull))
            })
            .collect::<Vec<ObjectId>>()
            .len() as i64) > 0
    } {
        let mut counters_2: Vec<(ObjectId, CounterKind, i32)> = Vec::new();
        for t in moved_1
            .clone()
            .get(0)
            .copied()
            .into_iter()
            .map(Target::Object)
            .collect::<Vec<Target>>()
        {
            if let Target::Object(object) = t {
                counters_2.push((object, CounterKind::Plus2Plus2, 1i32));
            }
        }
        let had_2: Vec<_> = counters_2
            .iter()
            .map(|c| g.state().obj(c.0).counters(c.1))
            .collect();
        g.add_counters_together(&counters_2);
        let applied_2: Vec<ObjectId> = counters_2
            .iter()
            .zip(had_2)
            .filter(|(c, had)| g.state().obj(c.0).counters(c.1) > *had)
            .map(|(c, _)| c.0)
            .collect();
        g.end_sentence();
        hoisted_3.extend(applied_2.clone());
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(
        &[target(creature_card)],
        as_additional_cost_cast_spell_2,
    ),
    additional_cost: Some(exile_permanents(1, as_additional_cost_cast_spell)),
    ..oracle::SOUL_EXCHANGE
};
