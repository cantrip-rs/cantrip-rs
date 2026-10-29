use cantrip_api::*;
use cantrip_mechanics::effects::may_return_first_target_to_hand;
use cantrip_mechanics::targets;
use cantrip_mechanics::triggers::THIS_ENTERS;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(THIS_ENTERS, may_return_first_target_to_hand, "When this creature enters, you may return target card from your graveyard to your hand.")
        .targets(&[targets::CARD_IN_YOUR_GRAVEYARD])],
    ..oracle::ETERNAL_WITNESS
};
