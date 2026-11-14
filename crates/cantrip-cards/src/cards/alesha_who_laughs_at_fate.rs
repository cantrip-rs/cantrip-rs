use cantrip_api::*;
use cantrip_mechanics::{effects, raid, targets, triggers};
use cantrip_oracle as oracle;

/// Put a +1/+1 counter on Alesha when she attacks, and at your end step after attacking return a creature card from your
/// graveyard whose mana value is at most her power then (CR 603.4: the "if you attacked" condition is checked as the end step
/// begins and again as the ability resolves; CR 608.2b: the target's mana value is judged against Alesha's power at resolution).
pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(triggers::THIS_ATTACKS, effects::plus_one_counter_on_this, "Whenever Alesha attacks, put a +1/+1 counter on it."),
        triggered(
            triggers::YOUR_END_STEP,
            effects::return_first_target_to_battlefield,
            "Raid \u{2014} At the beginning of your end step, if you attacked this turn, return target creature card with mana value less than or equal to Alesha's power from your graveyard to the battlefield.",
        )
        .targets(&[target(targets::creature_card_in_your_graveyard_up_to_source_power)])
        .condition(raid::you_attacked_this_turn),
    ],
    ..oracle::ALESHA_WHO_LAUGHS_AT_FATE
};
