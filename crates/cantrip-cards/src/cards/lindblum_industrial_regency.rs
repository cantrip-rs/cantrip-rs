use cantrip_api::*;
use cantrip_mechanics::{mana, tokens};
use cantrip_oracle as oracle;

/// The Adventure (CR 715): an instant, cast from the hand whenever an instant could be, then exiled on its adventure, from
/// where the land may be played (CR 715.3d). Its "Create a 0/1 black Wizard creature token with ..." is the shared token (CR 111.1).
pub static MAGE_SIEGE: CardDef = CardDef { spell: spell(&[], tokens::create_a_wizard), ..oracle::MAGE_SIEGE };

pub static CARD: CardDef = CardDef {
    // "This land enters tapped" (CR 614.1d).
    enters: Some(mana::enters_tapped),
    abilities: &[mana_ability(TAP, add(Mana::R), "{T}: Add {R}.")],
    adventure: Some(&MAGE_SIEGE),
    ..oracle::LINDBLUM_INDUSTRIAL_REGENCY
};

pub static EXTRA: &[&CardDef] = &[&MAGE_SIEGE, &tokens::WIZARD];
