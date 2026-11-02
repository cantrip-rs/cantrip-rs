use cantrip_api::*;
use cantrip_mechanics::effects;
use cantrip_oracle as oracle;

/// "When this card is put into your graveyard from your library" (CR 603.2).
fn when_card_put_into_graveyard_library(
    q: &dyn Query,
    asker: &Asker,
    event: &Event,
) -> Option<Fired> {
    let Event::ZoneChange { new, from, to, .. } = event else { return None };
    let id = *new;
    let subject = Target::Object(id);
    let data = [new.0, u32::MAX];
    if to.is(ZoneKind::Graveyard) && from.is(ZoneKind::Library)
        && (asker.source == id && asker.controller == q.state().obj(id).owner)
        && !q.state().obj(id).token
    {
        return Some(Fired { subject, data });
    }
    None
}

/// "When this card is put into your graveyard from your library, you may put it onto the battlefield." (CR 608.2).
fn put_onto_battlefield(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let mut hoisted_3: Vec<ObjectId> = Vec::new();
    let mut hoisted_2: Vec<ObjectId> = Vec::new();
    if ({
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
    })
        .is_some()
        && effects::ask_may(g, item.controller, item.object, Purpose::Optional)?
    {
        let objects_1 = ({
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
        hoisted_2.extend(moved_1.clone());
        hoisted_3
            .extend({
                let q: &dyn Query = g;
                objects_1
                    .iter()
                    .copied()
                    .filter(|&o| q.state().obj(o).became.is_some())
                    .collect::<Vec<ObjectId>>()
            });
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
                when_asked(EventMask::ZONE_CHANGE, when_card_put_into_graveyard_library)
                    .functions_in(ZoneKinds::of(&[ZoneKind::Graveyard])),
                put_onto_battlefield,
                "When this card is put into your graveyard from your library, you may put it onto the battlefield.",
            )
            .functions_in(ZoneKinds::of(&[ZoneKind::Graveyard])),
    ],
    ..oracle::NARCOMOEBA
};
