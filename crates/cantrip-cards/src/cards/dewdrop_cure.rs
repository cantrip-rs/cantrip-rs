use cantrip_api::*;
use cantrip_mechanics::{effects, gift, targets};
use cantrip_oracle as oracle;

/// "Return up to two target creature cards each with mana value 2 or less from your graveyard to the battlefield. If the
/// gift was promised, instead return up to three" (CR 110.2a, 608.2b-c): the slot holds two or, promised, three targets
/// (`TargetSpec::if_promised`); the ones still legal enter under your control, all at once.
pub static CARD: CardDef = CardDef {
    spell: spell(
        &[targets::CREATURE_CARD_WITH_MANA_VALUE_2_OR_LESS_IN_YOUR_GRAVEYARD.between(0, 2).if_promised(0, 3)],
        effects::return_targets_to_battlefield,
    ),
    gift: Some(gift::CARD),
    ..oracle::DEWDROP_CURE
};
