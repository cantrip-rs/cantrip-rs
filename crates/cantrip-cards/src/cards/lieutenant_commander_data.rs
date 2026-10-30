use cantrip_api::*;
use cantrip_mechanics::{effects, targets, triggers};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            triggers::A_PLAYER_CASTS_A_NONCREATURE_SPELL,
            effects::plus_one_counter_on_this,
            "Whenever a player casts a noncreature spell, put a +1/+1 counter on Lieutenant Commander Data.",
        ),
        ability(
            cost("{1}").and(remove_counters_among(3, targets::filters::creature)),
            |g, item| effects::draw(g, item, 1),
            "{1}, Remove three counters from among creatures you control: Draw a card.",
        ),
    ],
    ..oracle::LIEUTENANT_COMMANDER_DATA
};
