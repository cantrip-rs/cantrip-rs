use cantrip_api::*;
use cantrip_mechanics::tokens;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    spell: spell(&[], tokens::create_goblins::<2>),
    ..oracle::DRAGON_FODDER
};

pub static EXTRA: &[&CardDef] = &[&tokens::GOBLIN];
