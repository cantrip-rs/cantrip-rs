use cantrip_api::*;
use cantrip_mechanics::{lands, tokens, triggers};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(triggers::THIS_ENTERS, tokens::create_a_food_token, "When this creature enters, create a Food token. (It's an artifact with \"{2}, {T}, Sacrifice this token: You gain 3 life.\")"),
        // "Whenever you sacrifice a Food" (CR 701.21a, 603.2c): each Food sacrificed triggers it, so two sacrificed at once make two Rats.
        triggered(
            when_asked(EventMask::ZONE_CHANGE, triggers::you_sacrifice_a_subtype::<{ lands::sub(Subtype::Food) }>),
            tokens::create_rats_that_cant_block::<1>,
            "Whenever you sacrifice a Food, create a 1/1 black Rat creature token with \"This token can't block.\"",
        )
        .on_sacrifice(),
    ],
    ..oracle::EXPERIMENTAL_CONFECTIONER
};

pub static EXTRA: &[&CardDef] = &[&tokens::FOOD, &tokens::RAT_CANT_BLOCK];
