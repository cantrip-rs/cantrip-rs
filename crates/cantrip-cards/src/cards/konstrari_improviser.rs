use cantrip_api::*;
use cantrip_mechanics::tokens;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    enters: Some(cantrip_mechanics::prepare::enters_prepared),
    prepare: Some(&tokens::SOUL_TETHER),
    ..oracle::KONSTRARI_IMPROVISER
};

pub static EXTRA: &[&CardDef] = &[&tokens::SOUL_TETHER, &tokens::HEARTWOOD];
