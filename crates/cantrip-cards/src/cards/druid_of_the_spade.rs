use cantrip_api::*;
use cantrip_mechanics::statics;
use cantrip_oracle as oracle;

const TEXT: &str = "As long as you control a token, this creature gets +2/+0 and has trample.";

/// "As long as you control a token" (CR 604.1, 111.1, 613.1f): the source itself, while its controller (as the control-changing
/// effects of layer 2 leave it) controls a token, whatever the token is. Both of the creature's effects ask it.
fn itself_while_you_control_a_token(q: &dyn LayerQuery, source: ObjectId, affected: ObjectId, _: &Chars) -> bool {
    let you = q.controller(source);
    source == affected && q.state().battlefield().iter().any(|&id| q.state().obj(id).token && q.controller(id) == you)
}

pub static CARD: CardDef = CardDef {
    statics: &[
        static_ability(Layer::Ability, itself_while_you_control_a_token, statics::have_trample, TEXT),
        static_ability(Layer::PtModify, itself_while_you_control_a_token, statics::plus_pt::<2, 0>, TEXT),
    ],
    ..oracle::DRUID_OF_THE_SPADE
};
