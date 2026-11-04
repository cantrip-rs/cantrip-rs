use cantrip_api::*;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

/// "{U}{U}: Tap target creature without flying." (CR 115.1).
fn tap_target_creature_without_flying_target_0(
    q: &dyn Query,
    _: &Asker,
    t: Target,
) -> bool {
    matches!(
        t, Target::Object(id) if targets::permanent(q, id) && ((q
        .chars(id).is(CardType::Creature) && ! (q.chars(id).keywords
        .has(Keyword::Flying))))
    )
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        ability(
                cost("{U}{U}"),
                effects::tap_targets,
                "{U}{U}: Tap target creature without flying.",
            )
            .targets(&[target(tap_target_creature_without_flying_target_0)]),
    ],
    ..oracle::FLOOD
};
