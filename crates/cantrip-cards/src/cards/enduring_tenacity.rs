use cantrip_api::*;
use cantrip_mechanics::{effects, enduring, targets, triggers};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(triggers::YOU_GAIN_LIFE, effects::target_opponent_loses_that_much_life, "Whenever you gain life, target opponent loses that much life.")
            .targets(&[targets::OPPONENT]),
        triggered(when(EventMask::ZONE_CHANGE, enduring::dies_as_a_creature), enduring::return_as_enchantment, "When Enduring Tenacity dies, if it was a creature, return it to the battlefield under its owner's control. It's an enchantment. (It's not a creature.)"),
    ],
    ..oracle::ENDURING_TENACITY
};
