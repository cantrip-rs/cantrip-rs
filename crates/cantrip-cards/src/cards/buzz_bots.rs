use cantrip_api::*;
use cantrip_mechanics::effects;
use cantrip_mechanics::triggers::THIS_DIES;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    // CR 700.4: "dies" is put into a graveyard from the battlefield; the ability looks back at the creature (CR 603.10a).
    abilities: &[triggered(THIS_DIES, |g, item| effects::draw(g, item, 1), "When this creature dies, draw a card.")],
    ..oracle::BUZZ_BOTS
};
