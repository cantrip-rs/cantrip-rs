use cantrip_api::*;
use cantrip_mechanics::{effects, statics};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[ability(cost("{1}{R}"), effects::this_gets::<1, 0>, "{1}{R}: This creature gets +1/+0 until end of turn.")],
    statics: &[static_ability(Layer::Ability, statics::itself_during_your_turn, statics::have_first_strike, "During your turn, this creature has first strike.")],
    ..oracle::SKILLED_BATTLECARVER
};
