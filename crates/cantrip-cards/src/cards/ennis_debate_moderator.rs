use cantrip_api::*;
use cantrip_mechanics::triggers::{THIS_ENTERS, YOUR_END_STEP};
use cantrip_mechanics::{delayed, effects, history, targets};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    // "Exile up to one other target creature you control. Return that card to the battlefield under its owner's control at the
    // beginning of the next end step" (CR 603.7): a delayed ability about the card exiled. A token exiled this way ceases to
    // exist, so there is nothing to return (CR 111.7, 400.7). The shared slot-0 body.
    abilities: &[
        triggered(
            THIS_ENTERS,
            delayed::exile_target_then_return_at_next_end_step_0,
            "When Ennis enters, exile up to one other target creature you control. Return that card to the battlefield under its owner's control at the beginning of the next end step.",
        )
        .targets(&[targets::OTHER_CREATURE_YOU_CONTROL.optional()]),
        triggered(
            YOUR_END_STEP,
            effects::plus_one_counter_on_this,
            "At the beginning of your end step, if one or more cards were put into exile this turn, put a +1/+1 counter on Ennis.",
        )
        .condition(history::a_card_was_put_into_exile_this_turn),
    ],
    delayed: &[delayed_trigger(delayed::return_subject_to_battlefield, "Return that card to the battlefield under its owner's control at the beginning of the next end step.")],
    ..oracle::ENNIS_DEBATE_MODERATOR
};
