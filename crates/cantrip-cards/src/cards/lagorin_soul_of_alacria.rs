use cantrip_api::*;
use cantrip_mechanics::{effects, saddle, targets, triggers};
use cantrip_oracle as oracle;

/// "Flying" comes with the generated keywords. "Saddle 1" (CR 702.171a) is the shared mechanic and "attacks while saddled" is
/// judged when Lagorin attacks (CR 702.171b). "Each of up to two target Mounts and/or Vehicles" is one slot of up to two targets
/// (CR 601.2c), any controller's, a Vehicle that is not a creature included; each still legal as it resolves gets the counter.
pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            triggers::THIS_ATTACKS_WHILE_SADDLED,
            effects::plus_one_counter_on_each_target,
            "Whenever Lagorin attacks while saddled, put a +1/+1 counter on each of up to two target Mounts and/or Vehicles.",
        )
        .targets(&[targets::MOUNT_OR_VEHICLE.between(0, 2)]),
        saddle::ability(1, "Saddle 1 (Tap any number of other creatures you control with total power 1 or more: This Mount becomes saddled until end of turn. Saddle only as a sorcery.)"),
    ],
    ..oracle::LAGORIN_SOUL_OF_ALACRIA
};
