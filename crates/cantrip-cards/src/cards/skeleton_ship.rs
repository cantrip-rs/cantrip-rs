use cantrip_api::*;
use cantrip_mechanics::{effects, lands, targets};
use cantrip_oracle as oracle;

/// The controller's lack of Islands creates a state trigger (CR 603.8).
fn no_islands(q: &dyn Query, asker: &Asker) -> bool {
    !lands::controls_land_type(q, asker.controller, q.words(asker.source).subtype(Subtype::Island))
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        state_triggered(no_islands, effects::sacrifice_self, "When you control no Islands, sacrifice Skeleton Ship."),
        ability(TAP, effects::minus_one_counters_on_target::<1>, "{T}: Put a -1/-1 counter on target creature.")
            .targets(&[targets::CREATURE]),
    ],
    ..oracle::SKELETON_SHIP
};
