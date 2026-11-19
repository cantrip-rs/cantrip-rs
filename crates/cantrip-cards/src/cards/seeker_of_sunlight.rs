use cantrip_api::*;
use cantrip_mechanics::explore;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[ability(cost("{2}{G}"), |g, item| explore(g, item.object), "{2}{G}: This creature explores. Activate only as a sorcery.").sorcery_speed()],
    ..oracle::SEEKER_OF_SUNLIGHT
};
