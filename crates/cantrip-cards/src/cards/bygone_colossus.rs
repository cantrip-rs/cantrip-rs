use cantrip_api::*;
use cantrip_mechanics::warp;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    ways: &[warp::way(mana("{3}"), "Warp {3}")],
    end_step_riders: warp::RIDERS,
    delayed: &[warp::EXILE],
    ..oracle::BYGONE_COLOSSUS
};
