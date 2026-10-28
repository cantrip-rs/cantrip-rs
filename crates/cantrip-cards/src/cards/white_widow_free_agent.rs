use cantrip_api::*;
use cantrip_mechanics::effects;
use cantrip_mechanics::targets;
use cantrip_mechanics::triggers::THIS_ENTERS;
use cantrip_oracle as oracle;

/// "When White Widow enters, choose one — • Put a +1/+1 counter on each of up to two target creatures. • Return target artifact or
/// enchantment card from your graveyard to your hand." (CR 700.2, 603.3c.) Each mode has the targets of its own (CR 700.2c): up to
/// two creatures, which may be none at all, and an artifact or enchantment card in its controller's graveyard.
static MODAL: ModalDef = modes(&[
    mode(effects::plus_one_counter_on_each_target, "Put a +1/+1 counter on each of up to two target creatures.")
        .targets(&[targets::CREATURE.between(0, 2)]),
    mode(effects::return_first_target_to_hand, "Return target artifact or enchantment card from your graveyard to your hand.")
        .targets(&[targets::ARTIFACT_OR_ENCHANTMENT_CARD_IN_YOUR_GRAVEYARD]),
]);

pub static CARD: CardDef = CardDef {
    abilities: &[modal_triggered(
        THIS_ENTERS,
        &MODAL,
        "When White Widow enters, choose one \u{2014} Put a +1/+1 counter on each of up to two target creatures; or return target artifact or enchantment card from your graveyard to your hand.",
    )],
    ..oracle::WHITE_WIDOW_FREE_AGENT
};
