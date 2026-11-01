use cantrip_api::*;
use cantrip_mechanics::{airbend_targets, effects, lands, targets, triggers};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(triggers::THIS_ENTERS, airbend_targets, "When Aang enters, airbend up to one other target nonland permanent.")
            .targets(&[up_to_one(targets::other_nonland_permanent_filter)]),
        triggered(
            when_asked(EventMask::SPELL_CAST, triggers::subtype_spell_you_cast::<{ lands::sub(Subtype::Lesson) }>),
            |g, item| effects::this_gains_until_end_of_turn(g, item, Keyword::Lifelink),
            "Whenever you cast a Lesson spell, Aang gains lifelink until end of turn.",
        ),
    ],
    ..oracle::AANG_THE_LAST_AIRBENDER
};
