use cantrip_api::*;
use cantrip_mechanics::cumulative_upkeep;
use cantrip_mechanics::cumulative_upkeep::Payment;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[cumulative_upkeep!(Payment::OpponentGainsLife(1), "Cumulative upkeep—An opponent gains 1 life. (At the beginning of your upkeep, put an age counter on this permanent, then sacrifice it unless you pay its upkeep cost for each age counter on it.)")],
    ..oracle::WALL_OF_SHARDS
};
