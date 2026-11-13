use cantrip_api::*;
use cantrip_mechanics::triggers;
use cantrip_oracle as oracle;

/// "Whenever a creature dealt damage by this creature this turn dies" (CR 603.2).
fn when_creature_dealt_damage_by_creature(
    q: &dyn Query,
    asker: &Asker,
    event: &Event,
) -> Option<Fired> {
    let Event::ZoneChange { old, new, from, to, .. } = event else { return None };
    let id = *old;
    let subject = Target::Object(id);
    let data = [new.0, u32::MAX];
    if to.is(ZoneKind::Graveyard) && from.is(ZoneKind::Battlefield)
        && (q.chars(id).is(CardType::Creature)
            && ({
                let history_candidate = id;
                q
                    .history()
                    .this_turn()
                    .iter()
                    .filter_map(|r: &Record| -> Option<Fired> {
                        let event = &r.event;
                        if let Event::Damage { source, target, amount, .. } = event {
                            let (source, amount) = (*source, *amount);
                            let subject = *target;
                            let data = [u32::from(amount), source.0];
                            if q.state().obj(source).zone.is(ZoneKind::Battlefield)
                                && asker.source == source
                                && matches!(
                                    * target, Target::Object(o) if history_candidate == o
                                )
                            {
                                return Some(Fired { subject, data });
                            }
                        }
                        None
                    })
                    .count() as i64
            }) >= 1)
    {
        return Some(Fired { subject, data });
    }
    None
}

/// "Sacrifice the creature" (CR 608.2, 701.21a).
fn delayed_4(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let objects = g.event_objects(item).to_vec();
    g.sacrifice(item.controller, &objects);
    Ok(())
}

/// "Whenever a creature dealt damage by this creature this turn dies, put that card onto the battlefield under your control at the beginning of the next end step." (CR 608.2).
fn delayed_1(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let objects_2 = ({
        let q: &dyn Query = g;
        delayed_1_capture_0(q, item)
            .first()
            .copied()
            .into_iter()
            .filter(|&id| {
                is_live(q, id) && q.state().obj(id).zone.kind() == ZoneKind::Graveyard
            })
            .collect::<Vec<ObjectId>>()
            .get(0)
            .copied()
            .into_iter()
            .collect::<Vec<ObjectId>>()
    })
        .into_iter()
        .filter(|&o: &ObjectId| !g.state().obj(o).token)
        .collect::<Vec<ObjectId>>();
    let moved_2 = {
        let rows_2: Vec<(ObjectId, PlayerId, bool)> = objects_2
            .iter()
            .map(|&o| (o, item.controller, false))
            .collect();
        let came_2 = g.put_all_onto_battlefield_as(&rows_2)?;
        came_2
            .into_iter()
            .filter(|&o| g.state().obj(o).zone.is(ZoneKind::Battlefield))
            .collect::<Vec<ObjectId>>()
    };
    g.end_sentence();
    let about: Vec<ObjectId> = moved_2
        .clone()
        .get(0)
        .copied()
        .into_iter()
        .collect::<Vec<ObjectId>>();
    g.delayed_trigger_about(
        DelayedWhen::WhenEvent,
        item.object,
        item.controller,
        AbilityRef::delayed(item.ability.card, 0),
        &about,
    );
    Ok(())
}

/// "Whenever a creature dealt damage by this creature this turn dies, put that card onto the battlefield under your control at the beginning of the next end step." (CR 608.2).
fn delayed_1_capture_0(q: &dyn Query, item: &StackItem) -> Vec<ObjectId> {
    about(q, item, 0)
}

/// "Whenever a creature dealt damage by this creature this turn dies, put that card onto the battlefield under your control at the beginning of the next end step. Sacrifice the creature when you lose control of this creature." (CR 608.2).
fn put_card_onto_battlefield_under(
    g: &mut dyn Game,
    item: &StackItem,
) -> Result<(), Suspend> {
    let about: Vec<ObjectId> = Some(ObjectId(item.data[0]))
        .filter(|o| *o != ObjectId::NONE)
        .into_iter()
        .collect::<Vec<ObjectId>>();
    g.delayed_trigger_about(
        DelayedWhen::Step {
            step: Step::End,
            on_turn_of: None,
        },
        item.object,
        item.controller,
        AbilityRef::delayed(item.ability.card, 1),
        &about,
    );
    Ok(())
}

/// "Whenever a creature dealt damage by this creature this turn dies, put that card onto the battlefield under your control at the beginning of the next end step." (CR 608.2).
fn about(q: &dyn Query, item: &StackItem, part: usize) -> Vec<ObjectId> {
    assert_eq!(part, 0, "one real-object list");
    q.event_objects(item).to_vec()
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            when_asked(EventMask::ZONE_CHANGE, when_creature_dealt_damage_by_creature),
            put_card_onto_battlefield_under,
            "Whenever a creature dealt damage by this creature this turn dies, put that card onto the battlefield under your control at the beginning of the next end step. Sacrifice the creature when you lose control of this creature.",
        ),
    ],
    delayed: &[
        triggered(
            triggers::YOU_LOSE_CONTROL_OF_THIS,
            delayed_4,
            "Sacrifice the creature when you lose control of this creature.",
        ),
        delayed_trigger(
            delayed_1,
            "Put that card onto the battlefield under your control at the beginning of the next end step.",
        ),
    ],
    ..oracle::SERAPH
};
