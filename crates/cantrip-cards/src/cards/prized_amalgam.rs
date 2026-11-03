use cantrip_api::*;
use cantrip_oracle as oracle;

/// "Whenever a creature enters" (CR 603.2).
fn when_creature_enters(q: &dyn Query, asker: &Asker, event: &Event) -> Option<Fired> {
    let Event::ZoneChange { new, from, to, .. } = event else { return None };
    let id = *new;
    let subject = Target::Object(id);
    if to.is(ZoneKind::Battlefield) && q.chars(id).is(CardType::Creature)
        && subject
            .object()
            .is_some_and(|candidate: ObjectId| {
                (from.is(ZoneKind::Graveyard)
                    && asker.controller == q.state().obj(candidate).owner)
                    || cast_at(q, event, candidate)
                        .is_some_and(|cast| {
                            Some(asker.controller)
                                .into_iter()
                                .collect::<Vec<PlayerId>>()
                                .contains(&cast.player) && cast.from == ZoneKind::Graveyard
                                && Some(asker.controller)
                                    .into_iter()
                                    .collect::<Vec<PlayerId>>()
                                    .contains(&q.state().obj(cast.spell).owner)
                        })
            })
    {
        return Fired::about(subject);
    }
    None
}

/// "Whenever a creature enters, if it entered from your graveyard or you cast it from your graveyard, return this card from your graveyard to the battlefield tapped at the beginning of the next end step." (CR 608.2).
fn delayed_1(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let objects_2 = Some(item.object)
        .into_iter()
        .filter(|&o: &ObjectId| !g.state().obj(o).token)
        .collect::<Vec<ObjectId>>();
    let rows_2: Vec<(ObjectId, PlayerId, bool)> = objects_2
        .iter()
        .map(|&o| (o, item.controller, true))
        .collect();
    g.put_all_onto_battlefield_as(&rows_2)?;
    Ok(())
}

/// "Whenever a creature enters, if it entered from your graveyard or you cast it from your graveyard, return this card from your graveyard to the battlefield tapped at the beginning of the next end step." (CR 608.2).
fn if_entered_graveyard_cast_graveyard(
    g: &mut dyn Game,
    item: &StackItem,
) -> Result<(), Suspend> {
    if !{
        let q: &dyn Query = g;
        item.subject
            .object()
            .is_some_and(|candidate: ObjectId| {
                (q
                    .history()
                    .all()
                    .iter()
                    .rev()
                    .find_map(|r| match r.event {
                        Event::ZoneChange { new, from, to, .. } if new == candidate && to.is(ZoneKind::Battlefield) => {
                            Some(from)
                        }
                        _ => None,
                    })
                    .is_some_and(|from| from.is(ZoneKind::Graveyard))
                    && item.controller == q.state().obj(candidate).owner)
                    || cast_of(q, candidate)
                        .is_some_and(|cast| {
                            Some(item.controller)
                                .into_iter()
                                .collect::<Vec<PlayerId>>()
                                .contains(&cast.player) && cast.from == ZoneKind::Graveyard
                                && Some(item.controller)
                                    .into_iter()
                                    .collect::<Vec<PlayerId>>()
                                    .contains(&q.state().obj(cast.spell).owner)
                        })
            })
    } {
        return Ok(());
    }
    g.delayed_trigger(
        DelayedWhen::Step {
            step: Step::End,
            on_turn_of: None,
        },
        item.object,
        item.controller,
        AbilityRef::delayed(item.ability.card, 0),
        Target::None,
    );
    Ok(())
}

/// "Whenever a creature enters, if it entered from your graveyard or you cast it from your graveyard, return this card from your graveyard to the battlefield tapped at the beginning of the next end step." (CR 608.2).
fn cast_of<'a>(q: &'a dyn Query, id: ObjectId) -> Option<&'a Cast> {
    let records = q.history().all();
    let spell = records
        .iter()
        .rev()
        .find_map(|r| match r.event {
            Event::ZoneChange { old, new, reason: ZoneReason::Resolve, .. } if new == id => Some(old),
            _ => None,
        })
        .unwrap_or(id);
    records
        .iter()
        .rev()
        .find_map(|r| match &r.event {
            Event::Cast(cast) if cast.spell == spell => Some(cast),
            _ => None,
        })
}

/// "Whenever a creature enters, if it entered from your graveyard or you cast it from your graveyard, return this card from your graveyard to the battlefield tapped at the beginning of the next end step." (CR 608.2).
fn cast_at<'a>(q: &'a dyn Query, event: &Event, id: ObjectId) -> Option<&'a Cast> {
    match event {
        Event::ZoneChange { old, new, reason: ZoneReason::Resolve, .. } if *new == id => {
            cast_of(q, *old)
        }
        _ => cast_of(q, id),
    }
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
                when_asked(EventMask::ZONE_CHANGE, when_creature_enters),
                if_entered_graveyard_cast_graveyard,
                "Whenever a creature enters, if it entered from your graveyard or you cast it from your graveyard, return this card from your graveyard to the battlefield tapped at the beginning of the next end step.",
            )
            .functions_in(ZoneKinds::of(&[ZoneKind::Graveyard])),
    ],
    delayed: &[
        delayed_trigger(
            delayed_1,
            "Whenever a creature enters, if it entered from your graveyard or you cast it from your graveyard, return this card from your graveyard to the battlefield tapped at the beginning of the next end step.",
        ),
    ],
    ..oracle::PRIZED_AMALGAM
};
