use cantrip_api::*;
use cantrip_mechanics::effects;
use cantrip_oracle as oracle;

/// "When Zacama enters" (CR 603.2).
fn when_zacama_enters(q: &dyn Query, asker: &Asker, event: &Event) -> Option<Fired> {
    let Event::ZoneChange { new, to, .. } = event else { return None };
    let id = *new;
    let subject = Target::Object(id);
    if to.is(ZoneKind::Battlefield) && asker.source == id
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

/// "When Zacama enters, if you cast it, untap all lands you control." (CR 608.2, 701.26a, 502.3).
fn if_cast_untap_all_lands(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
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
    let objects_0 = {
        let q: &dyn Query = g;
        q.state()
            .battlefield()
            .iter()
            .copied()
            .filter(|&id| {
                q.chars(id).is(CardType::Land) && item.controller == q.controller(id)
            })
            .collect::<Vec<ObjectId>>()
            .into_iter()
            .filter(|&o| q.state().obj(o).became.is_none())
            .collect::<Vec<ObjectId>>()
    };
    let turned_0: Vec<ObjectId> = objects_0
        .iter()
        .copied()
        .filter(|&o| g.state().obj(o).tapped != false)
        .collect();
    instruction(
        g,
        |g| {
            for &object in &turned_0 {
                g.untap(object);
            }
        },
    );
    Ok(())
}

/// "{2}{R}: Zacama deals 3 damage to target creature." (CR 115.1).
fn zacama_deals_3_damage_target_target_0(q: &dyn Query, _: &Asker, t: Target) -> bool {
    matches!(
        t, Target::Object(id) if cantrip_mechanics::targets::permanent(q, id) && (q
        .chars(id).is(CardType::Creature))
    )
}

/// "{2}{R}: Zacama deals 3 damage to target creature." (CR 608.2, 120.3).
fn zacama_deals_3_damage_target(
    g: &mut dyn Game,
    item: &StackItem,
) -> Result<(), Suspend> {
    let t_0 = vec![g.state().target(item, 0)];
    let recipients_1 = t_0
        .get(0)
        .copied()
        .and_then(|t: Target| t.object())
        .into_iter()
        .map(Target::Object)
        .collect::<Vec<Target>>();
    let batch: Vec<(ObjectId, Target, u16)> = recipients_1
        .iter()
        .map(|t| (item.object, *t, 3u16))
        .collect();
    g.damage_together(&batch);
    Ok(())
}

/// "{2}{G}: Destroy target artifact or enchantment." (CR 115.1).
fn destroy_target_artifact_enchantment_target_0(
    q: &dyn Query,
    _: &Asker,
    t: Target,
) -> bool {
    matches!(
        t, Target::Object(id) if cantrip_mechanics::targets::permanent(q, id) && ((q
        .chars(id).is(CardType::Artifact) || q.chars(id).is(CardType::Enchantment)))
    )
}

/// "{2}{W}: You gain 3 life." (CR 608.2, 119.3).
fn gain_3_life(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    g.gain_life(item.controller, 3);
    Ok(())
}

/// "When Zacama enters, if you cast it, untap all lands you control." (CR 608.2).
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

/// "When Zacama enters, if you cast it, untap all lands you control." (CR 608.2).
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
            when_asked(EventMask::ZONE_CHANGE, when_zacama_enters),
            if_cast_untap_all_lands,
            "When Zacama enters, if you cast it, untap all lands you control.",
        ),
        ability(
                cost("{2}{R}"),
                zacama_deals_3_damage_target,
                "{2}{R}: Zacama deals 3 damage to target creature.",
            )
            .targets(&[target(zacama_deals_3_damage_target_target_0)]),
        ability(
                cost("{2}{G}"),
                effects::destroy_first_target,
                "{2}{G}: Destroy target artifact or enchantment.",
            )
            .targets(&[target(destroy_target_artifact_enchantment_target_0)]),
        ability(cost("{2}{W}"), gain_3_life, "{2}{W}: You gain 3 life."),
    ],
    ..oracle::ZACAMA_PRIMAL_CALAMITY
};
