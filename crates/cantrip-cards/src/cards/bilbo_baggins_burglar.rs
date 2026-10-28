use cantrip_api::*;
use cantrip_mechanics::effects;
use cantrip_mechanics::triggers::THIS_ENTERS;
use cantrip_oracle as oracle;

/// The Adventure (CR 715): "Scry 2." (CR 701.22).
pub static TAKE_A_GLANCE: CardDef = CardDef { spell: spell(&[], effects::scry_two), ..oracle::TAKE_A_GLANCE };

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(THIS_ENTERS, |g, item| effects::draw(g, item, 1), "When Bilbo Baggins enters, draw a card.")],
    adventure: Some(&TAKE_A_GLANCE),
    ..oracle::BILBO_BAGGINS_BURGLAR
};

pub static EXTRA: &[&CardDef] = &[&TAKE_A_GLANCE];
