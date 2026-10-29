use cantrip_api::*;
use cantrip_mechanics::triggers::THIS_ATTACKS;
use cantrip_mechanics::{effects, statics};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        THIS_ATTACKS,
        |g, item| effects::may_discard_to_draw(g, item, 1),
        "Whenever this creature attacks, you may discard a card. If you do, draw a card.",
    )],
    statics: &[static_ability(Layer::Ability, statics::itself_during_your_turn, statics::have_first_strike, "During your turn, this creature has first strike.")],
    ..oracle::NULL_GROUP_BIOLOGICAL_ASSETS
};
