use cantrip_api::*;
use cantrip_mechanics::triggers::SQUAD;
use cantrip_oracle as oracle;

/// "Squad\u{2014}Exile four cards from your graveyard." (CR 702.157a): the additional cost is paid any number of times as it
/// is cast, four cards from the graveyard for each.
pub static CARD: CardDef = CardDef {
    kicker: multikicker_cost(exile_from_graveyard(4)),
    abilities: &[SQUAD],
    ..oracle::RUTHLESS_RADRAT
};
