use cantrip_api::*;
use cantrip_mechanics::tokens::{self, RABBIT};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    spell: spell(&[], tokens::create_rabbits::<3>),
    ..oracle::HOP_TO_IT
};

pub static EXTRA: &[&CardDef] = &[&RABBIT];
