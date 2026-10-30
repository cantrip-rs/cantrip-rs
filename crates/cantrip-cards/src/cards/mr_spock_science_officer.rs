use cantrip_api::*;
use cantrip_mechanics::replacements::DRAW_TWO_INSTEAD;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    replacements: &[DRAW_TWO_INSTEAD],
    ..oracle::MR_SPOCK_SCIENCE_OFFICER
};
