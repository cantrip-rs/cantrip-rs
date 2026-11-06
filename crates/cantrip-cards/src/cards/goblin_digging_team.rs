use cantrip_api::*;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

/// "{T}, Sacrifice this creature: Destroy target Wall." (CR 115.1).
fn destroy_target_wall_target_0(q: &dyn Query, asker: &Asker, t: Target) -> bool {
    targets::permanent_of_any_subtype(q, asker, t, &[Subtype::Wall])
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        ability(
                SACRIFICE_THIS.and(TAP),
                effects::destroy_first_target,
                "{T}, Sacrifice this creature: Destroy target Wall.",
            )
            .targets(&[target(destroy_target_wall_target_0)]),
    ],
    ..oracle::GOBLIN_DIGGING_TEAM
};
