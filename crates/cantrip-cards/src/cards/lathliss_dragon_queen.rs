use cantrip_api::*;
use cantrip_mechanics::{effects, lands, tokens, triggers};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            when_asked(EventMask::ZONE_CHANGE, triggers::another_nontoken_of_subtype_you_control_enters::<{ lands::sub(Subtype::Dragon) }>),
            tokens::create_a_dragon_5_5,
            "Whenever another nontoken Dragon you control enters, create a 5/5 red Dragon creature token with flying.",
        ),
        ability(cost("{1}{R}"), effects::dragons_you_control_get_plus_n_zero::<1>, "{1}{R}: Dragons you control get +1/+0 until end of turn."),
    ],
    ..oracle::LATHLISS_DRAGON_QUEEN
};

pub static EXTRA: &[&CardDef] = &[&tokens::DRAGON_5_5];
