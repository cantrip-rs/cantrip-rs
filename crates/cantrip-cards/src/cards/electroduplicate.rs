use cantrip_api::*;
use cantrip_mechanics::copy::{self, SACRIFICE_THIS_TOKEN_EXCEPTION};
use cantrip_mechanics::{flashback, targets};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    spell: spell(&[targets::CREATURE_YOU_CONTROL], copy::create_token_copy_with_haste_and_sacrifice),
    ways: &[flashback::way(cost("{2}{R}{R}"), "Flashback {2}{R}{R}")],
    ..oracle::ELECTRODUPLICATE
};

pub static EXTRA: &[&CardDef] = &[&SACRIFICE_THIS_TOKEN_EXCEPTION];
