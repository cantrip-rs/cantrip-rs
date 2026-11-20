use cantrip_api::*;
use cantrip_mechanics::{effects, triggers};
use cantrip_oracle as oracle;

/// "... you may put a land card from your hand onto the battlefield tapped" (`effects::may_put_a_land_from_hand_tapped`): any land card
/// of the hand, the one the explore just put there among them (the explore is over before the trigger resolves, CR 701.44a, 603.3), or
/// none.
pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            triggers::CREATURE_YOU_CONTROL_EXPLORES_A_LAND_CARD,
            effects::may_put_a_land_from_hand_tapped,
            "Whenever a creature you control explores a land card, you may put a land card from your hand onto the battlefield tapped.",
        ),
        triggered(
            triggers::CREATURE_YOU_CONTROL_EXPLORES_A_NONLAND_CARD,
            effects::plus_one_counter_on_this,
            "Whenever a creature you control explores a nonland card, put a +1/+1 counter on Nicanzil.",
        ),
    ],
    ..oracle::NICANZIL_CURRENT_CONDUCTOR
};
