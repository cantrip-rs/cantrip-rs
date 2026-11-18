use cantrip_api::*;
use cantrip_mechanics::triggers::THIS_ENTERS;
use cantrip_mechanics::{effects, tokens};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(THIS_ENTERS, tokens::create_gnomes::<2>, "When this artifact enters, create two 1/1 colorless Gnome artifact creature tokens."),
        ability(cost("{W}").and(SACRIFICE_THIS), |g, item| effects::gain_life(g, item, 3), "{W}, Sacrifice this artifact: You gain 3 life."),
    ],
    ..oracle::TINKERS_TOTE
};

pub static EXTRA: &[&CardDef] = &[&tokens::GNOME];
