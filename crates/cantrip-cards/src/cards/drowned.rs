use cantrip_api::*;
use cantrip_mechanics::regenerate;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[ability(cost("{B}"), regenerate::this_effect, "{B}: Regenerate this creature.")],
    ..oracle::DROWNED
};
