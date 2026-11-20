use cantrip_api::*;
use cantrip_mechanics::statics::{have_trample, plus_pt};
use cantrip_mechanics::history;
use cantrip_oracle as oracle;

/// "Celebration — This creature gets +1/+1 and has trample as long as two or more nonland permanents entered the battlefield under
/// your control this turn" (CR 207.2c, 604.1, 611.3a, 613.4c, 613.6): one ability with an effect in layer 6 (trample) and one in
/// layer 7c (+1/+1), each asking the celebration again whenever characteristics are computed. The layer-7c part continues the
/// layer-6 one (`continues`), applying to what that part applied to.
const TEXT: &str = "Celebration \u{2014} This creature gets +1/+1 and has trample as long as two or more nonland permanents entered the battlefield under your control this turn.";

pub static CARD: CardDef = CardDef {
    statics: &[
        static_ability(Layer::Ability, history::this_while_you_celebrate, have_trample, TEXT),
        static_ability(Layer::PtModify, history::this_while_you_celebrate, plus_pt::<1, 1>, TEXT).continues(),
    ],
    ..oracle::GRAND_BALL_GUEST
};
