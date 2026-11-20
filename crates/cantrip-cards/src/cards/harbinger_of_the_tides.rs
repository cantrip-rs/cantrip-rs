use cantrip_api::*;
use cantrip_mechanics::effects::may_return_first_target_to_hand;
use cantrip_mechanics::triggers::THIS_ENTERS;
use cantrip_mechanics::{flash_cost, targets};
use cantrip_oracle as oracle;

/// "You may cast this spell as though it had flash if you pay {2} more to cast it" (CR 601.2b, 601.2f): the optional cost
/// `flash_cost` gives. "When this creature enters, you may return target tapped creature an opponent controls to its owner's
/// hand" (CR 603.1, 608.2b): the target is chosen as the trigger is put on the stack, and the may is asked as it resolves.
pub static CARD: CardDef = CardDef {
    optional_cost: Some(flash_cost::OPTIONAL_COST),
    labels: flash_cost::LABELS,
    abilities: &[triggered(
        THIS_ENTERS,
        may_return_first_target_to_hand,
        "When this creature enters, you may return target tapped creature an opponent controls to its owner's hand.",
    )
    .targets(&[targets::TAPPED_CREATURE_AN_OPPONENT_CONTROLS])],
    ..oracle::HARBINGER_OF_THE_TIDES
};
