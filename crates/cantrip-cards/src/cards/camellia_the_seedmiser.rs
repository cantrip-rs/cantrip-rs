use cantrip_api::*;
use cantrip_mechanics::forage;
use cantrip_mechanics::{effects, lands, statics, tokens, triggers};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    statics: &[static_ability(
        Layer::Ability,
        statics::other_of_subtype_you_control::<{ lands::sub(Subtype::Squirrel) }>,
        statics::have_menace,
        "Other Squirrels you control have menace.",
    )],
    abilities: &[
        // "One or more Foods" sacrificed at once (CR 603.2c) trigger it once.
        triggered(
            when_asked(EventMask::ZONE_CHANGE, triggers::you_sacrifice_a_subtype::<{ lands::sub(Subtype::Food) }>),
            tokens::create_a_squirrel,
            "Whenever you sacrifice one or more Foods, create a 1/1 green Squirrel creature token.",
        )
        .on_sacrifice()
        .once_per_batch(),
        // "Forage" is one of two costs the player chooses (CR 701.61a, 601.2b), beside {2}.
        ability(
            cost("{2}"),
            effects::plus_one_counter_on_each_other_of_subtype_you_control::<{ lands::sub(Subtype::Squirrel) }>,
            "{2}, Forage: Put a +1/+1 counter on each other Squirrel you control.",
        )
        .pay_one_of(&[forage::EXILE_THREE_CARDS, forage::SACRIFICE_A_FOOD]),
    ],
    ..oracle::CAMELLIA_THE_SEEDMISER
};

pub static EXTRA: &[&CardDef] = &[&tokens::SQUIRREL];
