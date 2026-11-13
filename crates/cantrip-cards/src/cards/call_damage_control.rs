use cantrip_api::*;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

/// "Choose up to two." (CR 700.2): the modes only name their targets, a card of that type in your graveyard each (CR 700.2c); the
/// instruction before them is done once to all of them (`ModalDef::shared`).
static MODAL: ModalDef = modes(&[
    mode(no_effect, "Target artifact card.").targets(&[targets::ARTIFACT_CARD_IN_YOUR_GRAVEYARD]),
    mode(no_effect, "Target creature card.").targets(&[targets::CREATURE_CARD_IN_YOUR_GRAVEYARD]),
    mode(no_effect, "Target enchantment card.").targets(&[targets::ENCHANTMENT_CARD_IN_YOUR_GRAVEYARD]),
    mode(no_effect, "Target land card.").targets(&[targets::LAND_CARD_IN_YOUR_GRAVEYARD]),
])
.choose(0, 2)
.shared(effects::return_each_target_to_hand);

pub static CARD: CardDef = CardDef { spell: Some(SpellDef::MODAL), modal: Some(&MODAL), ..oracle::CALL_DAMAGE_CONTROL };
