use cantrip_api::*;
use cantrip_mechanics::dash;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    statics: &[dash::HASTE],
    ways: &[dash::way(mana("{1}{R}"), "Dash {1}{R}")],
    end_step_riders: dash::RIDERS,
    delayed: &[dash::RETURN],
    ..oracle::SCREAMREACH_BRAWLER
};
