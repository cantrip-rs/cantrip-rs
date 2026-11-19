use cantrip_api::*;
use cantrip_mechanics::{copy, effects, targets, tokens};
use cantrip_oracle as oracle;

/// The 1/1 white Mouse creature token.
pub static MOUSE: CardDef = CardDef {
    colors: Some(ColorSet::of(Color::White)),
    types: TypeSet::of(&[CardType::Creature]),
    power: Some(1),
    toughness: Some(1),
    ..token!(Mouse)
};

pub static CARD: CardDef = CardDef {
    abilities: &[
        chapter(1, |g, item| tokens::create(g, item.controller, &MOUSE, 1).map(|_| ()), "I \u{2014} Create a 1/1 white Mouse creature token."),
        chapter(2, copy::create_token_copy_of_target, "II, III \u{2014} Create a token that's a copy of target token you control.")
            .targets(&[targets::TOKEN_YOU_CONTROL]),
        chapter(3, copy::create_token_copy_of_target, "II, III \u{2014} Create a token that's a copy of target token you control.")
            .targets(&[targets::TOKEN_YOU_CONTROL]),
        chapter(
            4,
            effects::creatures_you_control_get_and_gain::<1, 1, { Keywords::of(&[Keyword::Vigilance]).0 }>,
            "IV \u{2014} Creatures you control get +1/+1 and gain vigilance until end of turn.",
        ),
    ],
    ..oracle::THREE_BLIND_MICE
};

pub static EXTRA: &[&CardDef] = &[&MOUSE];
