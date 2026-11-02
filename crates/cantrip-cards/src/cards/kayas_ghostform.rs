use cantrip_api::*;
use cantrip_oracle as oracle;

/// "When enchanted permanent dies or is put into exile, return that card to the battlefield under your control." (CR 608.2).
fn return_card_battlefield_under_control_enchant(
    q: &dyn Query,
    asker: &Asker,
    t: Target,
) -> bool {
    matches!(
        t, Target::Object(id) if cantrip_mechanics::targets::permanent(q, id) && (((q
        .chars(id).is(CardType::Creature) || q.chars(id).is(CardType::Planeswalker)) &&
        asker.controller == q.controller(id)))
    )
}

/// "When enchanted permanent dies or is put into exile" (CR 603.2, 701.3a).
fn when_enchanted_permanent_dies_put_into(
    q: &dyn Query,
    asker: &Asker,
    event: &Event,
) -> Option<Fired> {
    let Event::ZoneChange { old, new, from, to, .. } = event else { return None };
    let id = *old;
    let subject = Target::Object(id);
    let data = [new.0, u32::MAX];
    if to.is(ZoneKind::Graveyard) && from.is(ZoneKind::Battlefield)
        && Some(asker.source).and_then(|o: ObjectId| q.state().obj(o).attached_to)
            == Some(id)
    {
        return Some(Fired { subject, data });
    }
    None
}

/// "When enchanted permanent dies or is put into exile, return that card to the battlefield under your control." (CR 608.2).
fn return_card_battlefield_under_control(
    g: &mut dyn Game,
    item: &StackItem,
) -> Result<(), Suspend> {
    let objects_0 = ({
        let q: &dyn Query = g;
        Some(ObjectId(item.data[0]))
            .filter(|o| *o != ObjectId::NONE)
            .into_iter()
            .filter(|&id| {
                q.state().obj(id).became.is_none()
                    && q.state().obj(id).zone.kind() == ZoneKind::Graveyard
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
    let rows_0: Vec<(ObjectId, PlayerId, bool)> = objects_0
        .iter()
        .map(|&o| (o, item.controller, false))
        .collect();
    g.put_all_onto_battlefield_as(&rows_0)?;
    Ok(())
}

/// "When enchanted permanent dies or is put into exile" (CR 603.2, 701.3a).
fn when_enchanted_permanent_dies_put_into_2(
    q: &dyn Query,
    asker: &Asker,
    event: &Event,
) -> Option<Fired> {
    let Event::ZoneChange { old, new, from, to, .. } = event else { return None };
    let id = *old;
    let subject = Target::Object(id);
    let data = [new.0, u32::MAX];
    if to.is(ZoneKind::Exile) && from.is(ZoneKind::Battlefield)
        && Some(asker.source).and_then(|o: ObjectId| q.state().obj(o).attached_to)
            == Some(id)
    {
        return Some(Fired { subject, data });
    }
    None
}

/// "When enchanted permanent dies or is put into exile, return that card to the battlefield under your control." (CR 608.2).
fn return_card_battlefield_under_control_2(
    g: &mut dyn Game,
    item: &StackItem,
) -> Result<(), Suspend> {
    let objects_1 = ({
        let q: &dyn Query = g;
        Some(ObjectId(item.data[0]))
            .filter(|o| *o != ObjectId::NONE)
            .into_iter()
            .filter(|&id| {
                q.state().obj(id).became.is_none()
                    && q.state().obj(id).zone.kind() == ZoneKind::Exile
            })
            .collect::<Vec<ObjectId>>()
            .get(0)
            .copied()
            .into_iter()
            .filter(|&id| {
                q.state().obj(id).became.is_none()
                    && q.state().obj(id).zone.kind() == ZoneKind::Exile
            })
            .collect::<Vec<ObjectId>>()
    })
        .into_iter()
        .filter(|&o: &ObjectId| !g.state().obj(o).token)
        .collect::<Vec<ObjectId>>();
    let rows_1: Vec<(ObjectId, PlayerId, bool)> = objects_1
        .iter()
        .map(|&o| (o, item.controller, false))
        .collect();
    g.put_all_onto_battlefield_as(&rows_1)?;
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            when_asked(EventMask::ZONE_CHANGE, when_enchanted_permanent_dies_put_into),
            return_card_battlefield_under_control,
            "When enchanted permanent dies or is put into exile, return that card to the battlefield under your control.",
        ),
        triggered(
            when_asked(EventMask::ZONE_CHANGE, when_enchanted_permanent_dies_put_into_2),
            return_card_battlefield_under_control_2,
            "When enchanted permanent dies or is put into exile, return that card to the battlefield under your control.",
        ),
    ],
    enchant: Some(target(return_card_battlefield_under_control_enchant)),
    ..oracle::KAYAS_GHOSTFORM
};
