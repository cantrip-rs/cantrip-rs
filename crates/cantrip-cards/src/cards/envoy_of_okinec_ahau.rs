use cantrip_api::*;
use cantrip_mechanics::tokens;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[ability(cost("{4}{W}"), tokens::create_gnomes::<1>, "{4}{W}: Create a 1/1 colorless Gnome artifact creature token.")],
    ..oracle::ENVOY_OF_OKINEC_AHAU
};

pub static EXTRA: &[&CardDef] = &[&tokens::GNOME];
