use cantrip_api::*;
use cantrip_mechanics::triggers::THIS_ENTERS;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(THIS_ENTERS, effects::return_first_target_to_hand, "When this Vehicle enters, return up to one other target nonland permanent to its owner's hand.")
            .targets(&[up_to_one(targets::other_nonland_permanent_filter)]),
        ability(cost("{3}"), effects::this_vehicle_becomes_artifact_creature_with_plus_one_counters::<3>, "Exhaust \u{2014} Waterbend {3}: This Vehicle becomes an artifact creature. Put three +1/+1 counters on it. (Activate each exhaust ability only once.)")
            .waterbend()
            .exhaust(),
    ],
    ..oracle::INVASION_SUBMERSIBLE
};
