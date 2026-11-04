use cantrip_api::*;
use cantrip_mechanics::effects;
use cantrip_oracle as oracle;

/// "{W}{W}: Return target nonsnow land you control to its owner's hand." (CR 115.1).
fn return_target_nonsnow_land_control_target_0(
    q: &dyn Query,
    asker: &Asker,
    t: Target,
) -> bool {
    (cantrip_mechanics::targets::LAND_YOU_CONTROL.filter)(q, asker, t)
        && matches!(t, Target::Object(id) if !q.chars(id).supertypes.snow)
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        ability(
                cost("{W}{W}"),
                effects::return_first_target_to_hand,
                "{W}{W}: Return target nonsnow land you control to its owner's hand.",
            )
            .targets(&[target(return_target_nonsnow_land_control_target_0)]),
    ],
    ..oracle::HALLOWED_GROUND
};
