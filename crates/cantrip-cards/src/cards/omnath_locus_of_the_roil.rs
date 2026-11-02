use cantrip_api::*;
use cantrip_mechanics::targets;
use cantrip_oracle as oracle;

/// "When Omnath enters" (CR 603.2).
fn when_omnath_enters(_: &dyn Query, asker: &Asker, event: &Event) -> Option<Fired> {
    let Event::ZoneChange { new, to, .. } = event else { return None };
    let id = *new;
    let subject = Target::Object(id);
    if to.is(ZoneKind::Battlefield) && asker.source == id {
        return Fired::about(subject);
    }
    None
}

/// "When Omnath enters, it deals damage to any target equal to the number of Elementals you control." (CR 608.2, 120.3).
fn deals_damage_any_target_equal(
    g: &mut dyn Game,
    item: &StackItem,
) -> Result<(), Suspend> {
    let t_0 = vec![g.state().target(item, 0)];
    let source_0 = item.subject.object();
    let amount_0 = {
        let q: &dyn Query = g;
        (q
            .state()
            .battlefield()
            .iter()
            .copied()
            .filter(|&id| {
                q.chars(id).has_subtype(q.words(item.object).subtype(Subtype::Elemental))
                    && item.controller == q.controller(id)
            })
            .collect::<Vec<ObjectId>>()
            .len() as i64) as u16
    };
    let recipients_0 = t_0.clone();
    let Some(source) = source_0 else { return Ok(()) };
    let batch: Vec<(ObjectId, Target, u16)> = recipients_0
        .iter()
        .map(|t| (source, *t, amount_0))
        .collect();
    g.damage_together(&batch);
    Ok(())
}

/// "Landfall — Whenever a land you control enters, put a +1/+1 counter on target Elemental you control. If you control eight or more lands, draw a card." (CR 115.1).
fn landfall_whenever_land_control_enters_target_0(
    q: &dyn Query,
    asker: &Asker,
    t: Target,
) -> bool {
    matches!(
        t, Target::Object(id) if cantrip_mechanics::targets::permanent(q, id) && ((q
        .chars(id).has_subtype(q.words(asker.source).subtype(Subtype::Elemental)) &&
        asker.controller == q.controller(id)))
    )
}

/// "Landfall — Whenever a land you control enters" (CR 603.2).
fn when_landfall_land_control_enters(
    q: &dyn Query,
    asker: &Asker,
    event: &Event,
) -> Option<Fired> {
    let Event::ZoneChange { new, to, .. } = event else { return None };
    let id = *new;
    let subject = Target::Object(id);
    if to.is(ZoneKind::Battlefield)
        && (q.chars(id).is(CardType::Land) && asker.controller == q.controller(id))
    {
        return Fired::about(subject);
    }
    None
}

/// "Landfall — Whenever a land you control enters, put a +1/+1 counter on target Elemental you control. If you control eight or more lands, draw a card." (CR 608.2, 121.1, 122.1).
fn landfall_whenever_land_control_enters(
    g: &mut dyn Game,
    item: &StackItem,
) -> Result<(), Suspend> {
    let t_0 = vec![g.state().target(item, 0)];
    let mut counters_1: Vec<(ObjectId, CounterKind, i16)> = Vec::new();
    for t in t_0
        .get(0)
        .copied()
        .and_then(|t: Target| t.object())
        .into_iter()
        .map(Target::Object)
        .collect::<Vec<Target>>()
    {
        if let Target::Object(object) = t {
            counters_1.push((object, CounterKind::PlusOne, 1i16));
        }
    }
    g.add_counters_together(&counters_1);
    g.end_sentence();
    if {
        let q: &dyn Query = g;
        (q
            .state()
            .battlefield()
            .iter()
            .copied()
            .filter(|&id| {
                q.chars(id).is(CardType::Land) && item.controller == q.controller(id)
            })
            .collect::<Vec<ObjectId>>()
            .len() as i64) >= 8
    } {
        g.draw(item.controller, 1);
        g.end_sentence();
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
                when_asked(EventMask::ZONE_CHANGE, when_omnath_enters),
                deals_damage_any_target_equal,
                "When Omnath enters, it deals damage to any target equal to the number of Elementals you control.",
            )
            .targets(&[targets::ANY]),
        triggered(
                when_asked(EventMask::ZONE_CHANGE, when_landfall_land_control_enters),
                landfall_whenever_land_control_enters,
                "Landfall — Whenever a land you control enters, put a +1/+1 counter on target Elemental you control. If you control eight or more lands, draw a card.",
            )
            .targets(&[target(landfall_whenever_land_control_enters_target_0)]),
    ],
    ..oracle::OMNATH_LOCUS_OF_THE_ROIL
};
