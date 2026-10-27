use cantrip_api::*;
use cantrip_mechanics::dash;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    statics: &[dash::HASTE],
    ways: &[dash::way(mana("{2}{B}"), "Dash {2}{B}")],
    end_step_riders: dash::RIDERS,
    delayed: &[dash::RETURN],
    ..oracle::ALESHAS_VANGUARD
};
