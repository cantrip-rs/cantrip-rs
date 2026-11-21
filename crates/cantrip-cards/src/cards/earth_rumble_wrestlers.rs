use cantrip_api::*;
use cantrip_mechanics::{history, statics};
use cantrip_oracle as oracle;

/// "As long as you control a land creature or a land entered the battlefield under your control this turn" (CR 604.1, 611.3a,
/// 613.1) as `StaticDef::affects`: only the source itself, and only while one of its controller's permanents is both a land and a
/// creature (as the earlier layers made it: an earthbent land counts, the source itself may be one) or a land entered under their
/// control this turn (CR 603.6a, whatever has happened to it since). The layers ask again whenever characteristics are computed, so
/// it turns on and off as lands arrive, animate and die.
fn itself_while_land_creature_or_land_entered(q: &dyn LayerQuery, source: ObjectId, affected: ObjectId, _: &Chars) -> bool {
    if affected != source {
        return false;
    }
    let state = q.state();
    let you = state.obj(source).controller;
    history::land_entered_under_control_this_turn(q.history(), you)
        || state.battlefield().iter().any(|&id| state.obj(id).controller == you && q.chars(id).is(CardType::Land) && q.chars(id).is(CardType::Creature))
}

pub static CARD: CardDef = CardDef {
    statics: &[
        static_ability(Layer::PtModify, itself_while_land_creature_or_land_entered, statics::plus_pt::<1, 0>, "This creature gets +1/+0 as long as you control a land creature or a land entered the battlefield under your control this turn."),
        static_ability(Layer::Ability, itself_while_land_creature_or_land_entered, statics::have_trample, "This creature has trample as long as you control a land creature or a land entered the battlefield under your control this turn."),
    ],
    ..oracle::EARTH_RUMBLE_WRESTLERS
};
