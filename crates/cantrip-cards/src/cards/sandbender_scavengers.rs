use cantrip_api::*;
use cantrip_mechanics::{effects, targets, triggers};
use cantrip_oracle as oracle;

const ASK: Question = Question::new("ExileSandbenderScavengers", "exile Sandbender Scavengers to return a creature card from your graveyard?", "Return a creature card with mana value up to its power", "Exile it", "Don't");

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            triggers::YOU_SACRIFICE_ANOTHER_PERMANENT,
            effects::plus_one_counter_on_this,
            "Whenever you sacrifice another permanent, put a +1/+1 counter on this creature.",
        )
        .on_sacrifice(),
        triggered(
            triggers::THIS_DIES,
            |g, item| effects::may_exile_it_when_you_do(g, item, &CARD, &ASK),
            "When this creature dies, you may exile it. When you do, return target creature card with mana value less than or equal to this creature's power from your graveyard to the battlefield.",
        ),
    ],
    delayed: &[delayed_trigger(
        effects::return_first_target_to_battlefield,
        "When you do, return target creature card with mana value less than or equal to this creature's power from your graveyard to the battlefield.",
    )
    .targets(&[target(targets::creature_card_in_your_graveyard_up_to_source_power)])],
    ..oracle::SANDBENDER_SCAVENGERS
};
