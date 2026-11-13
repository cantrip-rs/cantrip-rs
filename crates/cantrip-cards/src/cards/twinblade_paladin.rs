use cantrip_api::*;
use cantrip_mechanics::effects::plus_one_counter_on_this;
use cantrip_mechanics::statics::{have_double_strike, itself_with_life_at_least};
use cantrip_mechanics::triggers::YOU_GAIN_LIFE;
use cantrip_oracle as oracle;

/// "Whenever you gain life" triggers once for each life-gain event, not for each point (the 2024-11-08 rulings). "As long as you
/// have 25 or more life, this creature has double strike" (CR 702.4, 613.1f): while its controller's life total is 25 or more,
/// asked again whenever characteristics are computed, so it is lost the moment the total drops below 25.
pub static CARD: CardDef = CardDef {
    abilities: &[triggered(YOU_GAIN_LIFE, plus_one_counter_on_this, "Whenever you gain life, put a +1/+1 counter on this creature.")],
    statics: &[static_ability(
        Layer::Ability,
        itself_with_life_at_least::<25>,
        have_double_strike,
        "As long as you have 25 or more life, this creature has double strike.",
    )],
    ..oracle::TWINBLADE_PALADIN
};
