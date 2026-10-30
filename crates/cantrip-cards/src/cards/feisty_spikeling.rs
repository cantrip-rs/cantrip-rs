use cantrip_api::*;
use cantrip_mechanics::statics;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    statics: &[static_ability(Layer::Ability, statics::itself_during_your_turn, statics::have_first_strike, "During your turn, this creature has first strike.")],
    ..oracle::FEISTY_SPIKELING
};
