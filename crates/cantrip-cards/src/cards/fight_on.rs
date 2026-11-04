use cantrip_api::*;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

/// "Return up to two target creature cards from your graveyard to your hand." (CR 115.1, 608.2b): none, one or two chosen; the
/// ones still legal return together.
pub static CARD: CardDef = CardDef {
    spell: spell(&[targets::CREATURE_CARD_IN_YOUR_GRAVEYARD.between(0, 2)], effects::return_targets_to_hand),
    ..oracle::FIGHT_ON
};
