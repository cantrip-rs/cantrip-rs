use cantrip_api::*;
use cantrip_mechanics::{effects, targets, triggers};
use cantrip_oracle as oracle;

/// "Survival — At the beginning of your second main phase, if this creature is tapped, return target creature card with mana
/// value 3 or less from your graveyard to your hand." Survival is an ability word (CR 207.2c): the intervening "if" (CR 603.4)
/// is checked when the second main phase begins and again as the ability resolves, a creature that has left being judged by
/// the status it last had on the battlefield (CR 603.4, 608.2h). The target is chosen as the ability goes on the stack
/// (CR 603.3d) and judged again on resolution (CR 608.2b).
pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        triggers::YOUR_SECOND_MAIN_PHASE,
        effects::return_first_target_to_hand,
        "Survival \u{2014} At the beginning of your second main phase, if this creature is tapped, return target creature card with mana value 3 or less from your graveyard to your hand.",
    )
    .condition(triggers::this_is_tapped)
    .targets(&[targets::CREATURE_CARD_WITH_MANA_VALUE_3_OR_LESS_IN_YOUR_GRAVEYARD])],
    ..oracle::SAVIOR_OF_THE_SMALL
};
