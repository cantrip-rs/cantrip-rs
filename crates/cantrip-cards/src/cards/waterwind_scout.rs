use cantrip_api::*;
use cantrip_mechanics::tokens;
use cantrip_mechanics::triggers;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(triggers::THIS_ENTERS, |g, item| tokens::create_map(g, item.controller), "When this creature enters, create a Map token.")],
    ..oracle::WATERWIND_SCOUT
};

pub static EXTRA: &[&CardDef] = &[&tokens::MAP];
