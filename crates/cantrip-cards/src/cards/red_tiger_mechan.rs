use cantrip_api::*;
use cantrip_mechanics::warp;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    ways: &[warp::way(mana("{1}{R}"), "Warp {1}{R}")],
    end_step_riders: warp::RIDERS,
    delayed: &[warp::EXILE],
    ..oracle::RED_TIGER_MECHAN
};
