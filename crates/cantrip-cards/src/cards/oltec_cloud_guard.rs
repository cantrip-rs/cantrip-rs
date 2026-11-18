use cantrip_api::*;
use cantrip_mechanics::{tokens, triggers::THIS_ENTERS};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(THIS_ENTERS, tokens::create_gnomes::<1>, "When this creature enters, create a 1/1 colorless Gnome artifact creature token.")],
    ..oracle::OLTEC_CLOUD_GUARD
};

pub static EXTRA: &[&CardDef] = &[&tokens::GNOME];
