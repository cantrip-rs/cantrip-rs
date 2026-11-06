use cantrip_api::*;
use cantrip_mechanics::effects;
use cantrip_oracle as oracle;

/// "{T}: Destroy target creature with islandwalk." (CR 115.1).
fn destroy_target_creature_islandwalk_target_0(
    q: &dyn Query,
    asker: &Asker,
    t: Target,
) -> bool {
    matches!(
        t, Target::Object(id) if cantrip_mechanics::targets::permanent(q, id) && ((q
        .chars(id).is(CardType::Creature) && Keyword::landwalk(q.words(asker.source)
        .subtype(Subtype::Island)).is_some_and(| walk | q.chars(id).keywords
        .matches(walk))))
    )
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        ability(
                TAP,
                effects::destroy_first_target,
                "{T}: Destroy target creature with islandwalk.",
            )
            .targets(&[target(destroy_target_creature_islandwalk_target_0)]),
    ],
    ..oracle::MERFOLK_ASSASSIN
};
