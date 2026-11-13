use cantrip_api::*;
use cantrip_mechanics::effects;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[
        ability(TAP, |g, item| effects::draw(g, item, 3), "{T}: Draw three cards."),
        ability(cost("{2}{U}{U}"), effects::return_this_to_hand, "{2}{U}{U}: Return Arcanis to its owner's hand."),
    ],
    ..oracle::ARCANIS_THE_OMNIPOTENT
};
