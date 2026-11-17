use cantrip_api::*;
use cantrip_mechanics::{effects, plot, triggers};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            triggers::THIS_ENTERS,
            plot::may_exile_a_cheap_nonland_card_from_hand_and_plot,
            "When Kellan Joins Up enters, you may exile a nonland card with mana value 3 or less from your hand. If you do, it becomes plotted.",
        ),
        triggered(
            triggers::LEGENDARY_CREATURE_YOU_CONTROL_ENTERS,
            effects::plus_one_counter_on_each_creature_you_control,
            "Whenever a legendary creature you control enters, put a +1/+1 counter on each creature you control.",
        ),
    ],
    ..oracle::KELLAN_JOINS_UP
};
