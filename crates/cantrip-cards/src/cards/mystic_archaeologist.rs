use cantrip_api::*;
use cantrip_mechanics::effects;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[ability(cost("{3}{U}{U}"), |g, item| effects::draw(g, item, 2), "{3}{U}{U}: Draw two cards.")],
    ..oracle::MYSTIC_ARCHAEOLOGIST
};
