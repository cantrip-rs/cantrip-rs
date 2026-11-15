use cantrip_api::*;
use cantrip_mechanics::{effects, mana};
use cantrip_mechanics::triggers::THIS_ENTERS;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(THIS_ENTERS, effects::you_may_mill::<3>, "When this creature enters, you may mill three cards. (You may put the top three cards of your library into your graveyard.)"),
        mana_ability(TAP, add_by(mana::add_green_equal_to_its_power), "{T}: Add an amount of {G} equal to this creature's power."),
    ],
    ..oracle::RAINVEIL_REJUVENATOR
};
