use cantrip_api::*;
use cantrip_mechanics::{effects, targets, tokens, triggers};
use cantrip_oracle as oracle;

pub static EXTRA: &[&CardDef] = &[&tokens::BEAST_3_3];

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            triggers::CREATURE_SPELL_YOU_CAST,
            tokens::create_a_3_3_beast,
            "Whenever you cast a creature spell, create a 3/3 green Beast creature token.",
        ),
        triggered(
            triggers::NONCREATURE_SPELL_YOU_CAST,
            effects::plus_one_counters_on_target::<3>,
            "Whenever you cast a noncreature spell, put three +1/+1 counters on target creature you control.",
        )
        .targets(&[targets::CREATURE_YOU_CONTROL]),
        triggered(
            triggers::LANDFALL,
            |g, item| effects::gain_life(g, item, 3),
            "Landfall \u{2014} Whenever a land you control enters, you gain 3 life.",
        ),
    ],
    ..oracle::PRIMEVAL_BOUNTY
};
