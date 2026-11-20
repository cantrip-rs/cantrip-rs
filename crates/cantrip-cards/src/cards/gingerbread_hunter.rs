use cantrip_api::*;
use cantrip_mechanics::triggers::THIS_ENTERS;
use cantrip_mechanics::{effects, targets, tokens};
use cantrip_oracle as oracle;

/// The Adventure (CR 715.3b, 715.3d): Puny Snack, "Target creature gets -2/-2 until end of turn."
pub static PUNY_SNACK: CardDef = CardDef { spell: spell(&[targets::CREATURE], effects::target_creature_gets::<-2, -2>), ..oracle::PUNY_SNACK };

pub static CARD: CardDef = CardDef {
    adventure: Some(&PUNY_SNACK),
    abilities: &[triggered(THIS_ENTERS, tokens::create_a_food_token, "When this creature enters, create a Food token.")],
    ..oracle::GINGERBREAD_HUNTER
};

pub static EXTRA: &[&CardDef] = &[&PUNY_SNACK, &tokens::FOOD];
