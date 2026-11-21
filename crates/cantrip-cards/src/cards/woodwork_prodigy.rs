use cantrip_api::*;
use cantrip_mechanics::{prepare, tokens};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[prepare::AT_YOUR_UPKEEP_IF_NOT_PREPARED_BECOMES_PREPARED],
    prepare: Some(&tokens::SOUL_TETHER),
    ..oracle::WOODWORK_PRODIGY
};

pub static EXTRA: &[&CardDef] = &[&tokens::SOUL_TETHER, &tokens::HEARTWOOD];
