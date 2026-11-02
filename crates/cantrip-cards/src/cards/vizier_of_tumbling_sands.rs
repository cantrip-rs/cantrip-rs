use cantrip_api::*;
use cantrip_oracle as oracle;

/// "{T}: Untap another target permanent." (CR 115.1).
fn untap_another_target_permanent_target_0(
    q: &dyn Query,
    asker: &Asker,
    t: Target,
) -> bool {
    matches!(
        t, Target::Object(id) if cantrip_mechanics::targets::permanent(q, id) && ((!
        (asker.source == id)))
    )
}

/// "{T}: Untap another target permanent." (CR 608.2, 701.26a, 502.3).
fn untap_another_target_permanent(
    g: &mut dyn Game,
    item: &StackItem,
) -> Result<(), Suspend> {
    let t_0 = vec![g.state().target(item, 0)];
    let objects_0 = {
        let q: &dyn Query = g;
        t_0.get(0)
            .copied()
            .and_then(|t: Target| t.object())
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

/// "Cycling {1}{U}" (CR 608.2, 121.1).
fn cycling_1_u(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    g.draw(item.controller, 1);
    Ok(())
}

/// "When you cycle this card, untap target permanent." (CR 115.1).
fn untap_target_permanent_target_0(q: &dyn Query, _: &Asker, t: Target) -> bool {
    matches!(
        t, Target::Object(id) if cantrip_mechanics::targets::permanent(q, id) && (true)
    )
}

/// "When you cycle this card" (CR 603.2).
fn when_cycle_card(_: &dyn Query, asker: &Asker, event: &Event) -> Option<Fired> {
    let Event::Cycled { card, player } = event else { return None };
    let id = *card;
    let subject = Target::Object(id);
    if asker.controller == *player && id == asker.source {
        return Fired::about(subject);
    }
    None
}

/// "When you cycle this card, untap target permanent." (CR 608.2, 701.26a, 502.3).
fn untap_target_permanent(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let t_0 = vec![g.state().target(item, 0)];
    let objects_2 = {
        let q: &dyn Query = g;
        t_0.get(0)
            .copied()
            .and_then(|t: Target| t.object())
            .into_iter()
            .filter(|&o| q.state().obj(o).became.is_none())
            .collect::<Vec<ObjectId>>()
    };
    let turned_2: Vec<ObjectId> = objects_2
        .iter()
        .copied()
        .filter(|&o| g.state().obj(o).tapped != false)
        .collect();
    instruction(
        g,
        |g| {
            for &object in &turned_2 {
                g.untap(object);
            }
        },
    );
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        ability(
                TAP,
                untap_another_target_permanent,
                "{T}: Untap another target permanent.",
            )
            .targets(&[target(untap_another_target_permanent_target_0)]),
        ability(cost("{1}{U}").and(DISCARD_THIS), cycling_1_u, "Cycling {1}{U}")
            .cycling()
            .from(ZoneKind::Hand),
        triggered(
                when_asked(EventMask::CYCLED, when_cycle_card)
                    .functions_in(WHERE_A_DISCARD_ENDS),
                untap_target_permanent,
                "When you cycle this card, untap target permanent.",
            )
            .targets(&[target(untap_target_permanent_target_0)]),
    ],
    ..oracle::VIZIER_OF_TUMBLING_SANDS
};
