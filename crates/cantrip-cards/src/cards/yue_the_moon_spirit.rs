use cantrip_api::*;
use cantrip_mechanics::choose;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[ability(
        cost("{5}, {T}"),
        |g, item| {
            choose::may_cast_from_hand_free(g, item.controller, item.object, "noncreature spell", |g, card, way| {
                !g.chars_as_cast(card, way.way, item.controller).is(CardType::Creature)
            })
        },
        "Waterbend {5}, {T}: You may cast a noncreature spell from your hand without paying its mana cost. (While paying a waterbend cost, you can tap your artifacts and creatures to help. Each one pays for {1}.)",
    )
    .waterbend()],
    ..oracle::YUE_THE_MOON_SPIRIT
};
