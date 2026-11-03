use cantrip_api::*;
use cantrip_mechanics::{effects, library, select, same_name};
use cantrip_oracle as oracle;

/// "Whenever an Aura you control enters" (CR 603.2).
fn when_aura_control_enters(
    q: &dyn Query,
    asker: &Asker,
    event: &Event,
) -> Option<Fired> {
    let Event::ZoneChange { new, to, .. } = event else { return None };
    let id = *new;
    let subject = Target::Object(id);
    if to.is(ZoneKind::Battlefield)
        && (q.chars(id).has_subtype(q.words(asker.source).subtype(Subtype::Aura))
            && asker.controller == q.controller(id))
        && subject
            .object()
            .is_some_and(|candidate: ObjectId| {
                cast_at(q, event, candidate)
                    .is_some_and(|cast| {
                        Some(asker.controller)
                            .into_iter()
                            .collect::<Vec<PlayerId>>()
                            .contains(&cast.player)
                    })
            })
    {
        return Fired::about(subject);
    }
    None
}

/// "Whenever an Aura you control enters, if you cast it, you may search your library for an Aura card with mana value less than or equal to that Aura and with a different name than each Aura you control, put that card onto the battlefield attached to Light-Paws, then shuffle." (CR 608.2, 701.24a).
fn if_cast_search_library_aura(
    g: &mut dyn Game,
    item: &StackItem,
) -> Result<(), Suspend> {
    if !{
        let q: &dyn Query = g;
        item.subject
            .object()
            .is_some_and(|candidate: ObjectId| {
                cast_of(q, candidate)
                    .is_some_and(|cast| {
                        Some(item.controller)
                            .into_iter()
                            .collect::<Vec<PlayerId>>()
                            .contains(&cast.player)
                    })
            })
    } {
        return Ok(());
    }
    let mut hoisted_6: Vec<ObjectId> = Vec::new();
    let mut hoisted_5: Vec<ObjectId> = Vec::new();
    let mut hoisted_4: Vec<ObjectId> = Vec::new();
    if g.may_search(item.controller, item.controller)
        && effects::ask_may(g, item.controller, item.object, Purpose::Optional)?
    {
        let shown_1 = library::searched_cards(g, item.controller);
        let can_1: Vec<ObjectId> = {
            let q: &dyn Query = g;
            shown_1
                .iter()
                .copied()
                .filter(|&id| {
                    q.chars(id).has_subtype(q.words(item.object).subtype(Subtype::Aura))
                        && (i64::from(q.mana_value(id)))
                            <= (item
                                .subject
                                .object()
                                .map_or(0, |o: ObjectId| i64::from(q.mana_value(o))))
                        && !(q
                            .state()
                            .battlefield()
                            .iter()
                            .copied()
                            .filter(|&id| {
                                q
                                    .chars(id)
                                    .has_subtype(q.words(item.object).subtype(Subtype::Aura))
                                    && item.controller == q.controller(id)
                            })
                            .collect::<Vec<ObjectId>>()
                            .iter()
                            .any(|&o| same_name(q, id, o)))
                })
                .collect()
        };
        let found_1 = library::search(
            g,
            item.controller,
            item.controller,
            item.object,
            SearchDestination::Battlefield,
            select::Selection::listed(select::Count::UpTo(1), &shown_1, can_1),
        )?;
        let objects_2 = found_1
            .clone()
            .into_iter()
            .filter(|&o: &ObjectId| !g.state().obj(o).token)
            .collect::<Vec<ObjectId>>();
        let moved_2 = {
            let mut moved: Vec<ObjectId> = Vec::new();
            for &o in &objects_2 {
                moved
                    .extend(
                        g.put_onto_battlefield_attached(o, item.controller, item.object)?,
                    );
            }
            moved
        };
        g.shuffle(item.controller);
        g.end_sentence();
        hoisted_4.extend(moved_2.clone());
        hoisted_5.extend(found_1.clone());
        hoisted_6
            .extend({
                let q: &dyn Query = g;
                objects_2
                    .iter()
                    .copied()
                    .filter(|&o| q.state().obj(o).became.is_some())
                    .collect::<Vec<ObjectId>>()
            });
    }
    Ok(())
}

/// "Whenever an Aura you control enters, if you cast it, you may search your library for an Aura card with mana value less than or equal to that Aura and with a different name than each Aura you control, put that card onto the battlefield attached to Light-Paws, then shuffle." (CR 608.2).
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

/// "Whenever an Aura you control enters, if you cast it, you may search your library for an Aura card with mana value less than or equal to that Aura and with a different name than each Aura you control, put that card onto the battlefield attached to Light-Paws, then shuffle." (CR 608.2).
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
            when_asked(EventMask::ZONE_CHANGE, when_aura_control_enters),
            if_cast_search_library_aura,
            "Whenever an Aura you control enters, if you cast it, you may search your library for an Aura card with mana value less than or equal to that Aura and with a different name than each Aura you control, put that card onto the battlefield attached to Light-Paws, then shuffle.",
        ),
    ],
    ..oracle::LIGHT_PAWS_EMPERORS_VOICE
};
