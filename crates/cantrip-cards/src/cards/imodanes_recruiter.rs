use cantrip_api::*;
use cantrip_mechanics::{effects, tokens, triggers};
use cantrip_oracle as oracle;

/// The Adventure (CR 715): Train Troops, "Create two 2/2 white Knight creature tokens with vigilance."
pub static TRAIN_TROOPS: CardDef = CardDef { spell: spell(&[], |g, item| tokens::create(g, item.controller, &tokens::KNIGHT, 2).map(|_| ())), ..oracle::TRAIN_TROOPS };

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        triggers::THIS_ENTERS,
        effects::creatures_you_control_get_and_gain::<1, 0, { Keywords::of(&[Keyword::Haste]).0 }>,
        "When this creature enters, creatures you control get +1/+0 and gain haste until end of turn.",
    )],
    adventure: Some(&TRAIN_TROOPS),
    ..oracle::IMODANES_RECRUITER
};

pub static EXTRA: &[&CardDef] = &[&TRAIN_TROOPS, &tokens::KNIGHT];
