use cantrip_api::*;
use cantrip_mechanics::triggers::THIS_ATTACKS_ALONE;
use cantrip_mechanics::{effects, targets, tokens};
use cantrip_oracle as oracle;

/// The Adventure (CR 715): Go Hog Wild.
pub static GO_HOG_WILD: CardDef = CardDef { spell: spell(&[targets::CREATURE], effects::target_creature_gets::<2, 2>), ..oracle::GO_HOG_WILD };

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(THIS_ATTACKS_ALONE, tokens::create_a_food_token, "Whenever this creature attacks alone, create a Food token.")],
    adventure: Some(&GO_HOG_WILD),
    ..oracle::INTREPID_TRUFFLESNOUT
};

pub static EXTRA: &[&CardDef] = &[&GO_HOG_WILD, &tokens::FOOD];
