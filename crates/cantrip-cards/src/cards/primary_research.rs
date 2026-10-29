use cantrip_api::*;
use cantrip_mechanics::triggers::{THIS_ENTERS, YOUR_END_STEP};
use cantrip_mechanics::{effects, history, targets};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(THIS_ENTERS, effects::return_target_card_to_battlefield, "When this enchantment enters, return target nonland permanent card with mana value 3 or less from your graveyard to the battlefield.")
            .targets(&[targets::NONLAND_PERMANENT_CARD_WITH_MANA_VALUE_3_OR_LESS_IN_YOUR_GRAVEYARD]),
        // "If a card left your graveyard this turn" (CR 603.4): asked of the history as the step begins and again as it resolves.
        triggered(YOUR_END_STEP, |g, item| effects::draw(g, item, 1), "At the beginning of your end step, if a card left your graveyard this turn, draw a card.")
            .condition(history::a_card_left_your_graveyard_this_turn),
    ],
    ..oracle::PRIMARY_RESEARCH
};
