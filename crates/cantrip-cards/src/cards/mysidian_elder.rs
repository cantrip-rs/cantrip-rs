use cantrip_api::*;
use cantrip_mechanics::tokens;
use cantrip_mechanics::triggers::THIS_ENTERS;
use cantrip_oracle as oracle;

/// "When this creature enters, create a 0/1 black Wizard creature token with 'Whenever you cast a noncreature spell, this token
/// deals 1 damage to each opponent.'" (CR 111.1, 603.6a).
pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        THIS_ENTERS,
        tokens::create_a_wizard,
        "When this creature enters, create a 0/1 black Wizard creature token with \"Whenever you cast a noncreature spell, this token deals 1 damage to each opponent.\"",
    )],
    ..oracle::MYSIDIAN_ELDER
};

pub static EXTRA: &[&CardDef] = &[&tokens::WIZARD];
