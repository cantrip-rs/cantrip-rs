use cantrip_api::*;
use cantrip_mechanics::targets;
use cantrip_oracle as oracle;

/// "{T}, Sacrifice a green creature: Target land becomes a Forest." (CR 608.2, 611.2a).
fn target_land_becomes_forest(
    g: &mut dyn Game,
    item: &StackItem,
) -> Result<(), Suspend> {
    let Some(target) = targets::object(g, item, 0) else { return Ok(()) };
    let land_type = g.words(item.object).subtype(Subtype::Forest);
    instruction(
        g,
        |g| {
            g.add_effect(
                item.object,
                target,
                EffectKind::SetLandType(land_type),
                Duration::Indefinite,
            );
        },
    );
    Ok(())
}

/// "{T}, Sacrifice a green creature: Target land becomes a Forest." (CR 118.1).
fn target_land_becomes_forest_2(q: &dyn Query, asker: &Asker, id: ObjectId) -> bool {
    q.chars(id).colors.has(q.words(asker.source).color(Color::Green))
        && q.chars(id).is(CardType::Creature)
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        ability(
                TAP.and(sacrifice(1, target_land_becomes_forest_2)),
                target_land_becomes_forest,
                "{T}, Sacrifice a green creature: Target land becomes a Forest.",
            )
            .targets(&[targets::LAND]),
    ],
    ..oracle::THELONITE_MONK
};
