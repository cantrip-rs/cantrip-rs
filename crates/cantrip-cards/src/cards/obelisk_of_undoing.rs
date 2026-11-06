use cantrip_api::*;
use cantrip_mechanics::effects;
use cantrip_oracle as oracle;

/// "{6}, {T}: Return target permanent you both own and control to your hand." (CR 115.1).
fn return_target_permanent_both_own_target_0(
    q: &dyn Query,
    asker: &Asker,
    t: Target,
) -> bool {
    matches!(
        t, Target::Object(id) if cantrip_mechanics::targets::permanent(q, id) && ((asker
        .controller == q.state().obj(id).owner && asker.controller == q.controller(id)))
    )
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        ability(
                cost("{6}").and(TAP),
                effects::return_first_target_to_hand,
                "{6}, {T}: Return target permanent you both own and control to your hand.",
            )
            .targets(&[target(return_target_permanent_both_own_target_0)]),
    ],
    ..oracle::OBELISK_OF_UNDOING
};
