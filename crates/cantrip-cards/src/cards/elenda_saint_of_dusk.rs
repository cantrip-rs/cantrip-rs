use cantrip_api::*;
use cantrip_mechanics::statics::plus_pt;
use cantrip_oracle as oracle;

/// How far its controller's life total is above the life total they started the game with (CR 103.4).
fn life_above_start(q: &dyn LayerQuery, source: ObjectId) -> i32 {
    let state = q.state();
    let player = state.player(state.obj(source).controller);
    player.life - player.starting_life
}

/// "As long as your life total is greater than your starting life total": only Elenda herself (CR 604.2, 611.3a).
fn above_starting_life(q: &dyn LayerQuery, source: ObjectId, affected: ObjectId, _: &Chars) -> bool {
    source == affected && life_above_start(q, source) > 0
}

/// "As long as your life total is at least 10 greater than your starting life total."
fn ten_above_starting_life(q: &dyn LayerQuery, source: ObjectId, affected: ObjectId, _: &Chars) -> bool {
    source == affected && life_above_start(q, source) >= 10
}

pub static CARD: CardDef = CardDef {
    // "Hexproof from instants" (CR 702.11d).
    hexproof_from: HexproofFrom::card_type(CardType::Instant),
    statics: &[
        static_ability(Layer::PtModify, above_starting_life, plus_pt::<1, 1>, "As long as your life total is greater than your starting life total, Elenda gets +1/+1."),
        static_ability(Layer::Ability, above_starting_life, cantrip_mechanics::statics::have_menace, "As long as your life total is greater than your starting life total, Elenda has menace."),
        static_ability(Layer::PtModify, ten_above_starting_life, plus_pt::<5, 5>, "Elenda gets an additional +5/+5 as long as your life total is at least 10 greater than your starting life total."),
    ],
    ..oracle::ELENDA_SAINT_OF_DUSK
};
