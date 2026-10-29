use cantrip_api::*;
use cantrip_mechanics::triggers::{self, THIS_ENTERS};
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(THIS_ENTERS, effects::return_first_target_to_hand, "When this creature enters, return up to one other target creature to its owner's hand.")
            .targets(&[up_to_one(targets::other_creature_filter)]),
        triggered(triggers::YOU_CAST_A_SPELL_WITH_X_IN_ITS_MANA_COST, effects::this_cant_be_blocked_this_turn, "Whenever you cast a spell with {X} in its mana cost, this creature can't be blocked this turn."),
    ],
    ..oracle::MATTERBENDING_MAGE
};
