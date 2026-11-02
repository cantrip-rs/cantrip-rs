use cantrip_api::*;
use cantrip_oracle as oracle;

/// "{T}: Untap target land." (CR 115.1).
fn untap_target_land_target_0(q: &dyn Query, _: &Asker, t: Target) -> bool {
    matches!(
        t, Target::Object(id) if cantrip_mechanics::targets::permanent(q, id) && (q
        .chars(id).is(CardType::Land))
    )
}

/// "{T}: Untap target land." (CR 608.2, 701.26a, 502.3).
fn untap_target_land(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
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

pub static CARD: CardDef = CardDef {
    abilities: &[
        ability(TAP, untap_target_land, "{T}: Untap target land.")
            .targets(&[target(untap_target_land_target_0)]),
    ],
    ..oracle::VOYAGING_SATYR
};
