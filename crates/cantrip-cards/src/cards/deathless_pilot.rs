use cantrip_api::*;
use cantrip_mechanics::effects;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    keywords: Keywords::of(&[Keyword::SaddlesAsThoughPowerTwoGreater, Keyword::CrewsAsThoughPowerTwoGreater]),
    abilities: &[ability(cost("{3}{B}"), effects::return_this_to_hand, "{3}{B}: Return this card from your graveyard to your hand.").from(ZoneKind::Graveyard)],
    ..oracle::DEATHLESS_PILOT
};
