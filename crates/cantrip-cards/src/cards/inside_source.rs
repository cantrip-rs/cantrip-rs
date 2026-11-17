use cantrip_api::*;
use cantrip_mechanics::triggers::THIS_ENTERS;
use cantrip_mechanics::{effects, lands, targets, tokens};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(THIS_ENTERS, tokens::create_a_white_blue_detective, "When this creature enters, create a 2/2 white and blue Detective creature token."),
        ability(
            cost("{3}").and(TAP),
            |g, item| effects::targets_get_and_gain(g, item, 2, 0, Keyword::Vigilance),
            "{3}, {T}: Target Detective you control gets +2/+0 and gains vigilance until end of turn. Activate only as a sorcery.",
        )
        .targets(&[target(targets::permanent_of_subtype_you_control::<{ lands::sub(Subtype::Detective) }>)])
        .sorcery_speed(),
    ],
    ..oracle::INSIDE_SOURCE
};
pub static EXTRA: &[&CardDef] = &[&tokens::WHITE_BLUE_DETECTIVE];
