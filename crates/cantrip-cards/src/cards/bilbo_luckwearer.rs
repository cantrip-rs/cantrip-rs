use cantrip_api::*;
use cantrip_mechanics::triggers::DEALS_COMBAT_DAMAGE_TO_PLAYER;
use cantrip_mechanics::{effects, exchange, select, targets};
use cantrip_oracle as oracle;

/// The Adventure (CR 715), "Exchange control of two target nonland permanents that share a card type" (CR 701.12a-b): two
/// different permanents (CR 601.2c) that still share a card type as it resolves (CR 608.2b), or none of it happens
/// (`TargetSpec::together` judges the pair again on resolution); two permanents one player controls exchange nothing.
pub static BURGLARS_PLOT: CardDef = CardDef {
    spell: spell(&[targets::NONLAND_PERMANENT.between(2, 2).together(select::SHARE_A_CARD_TYPE)], exchange::control_of_two_targets),
    ..oracle::BURGLARS_PLOT
};

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(DEALS_COMBAT_DAMAGE_TO_PLAYER, effects::loot, "Whenever Bilbo deals combat damage to a player, draw a card, then discard a card.")],
    adventure: Some(&BURGLARS_PLOT),
    ..oracle::BILBO_LUCKWEARER
};

pub static EXTRA: &[&CardDef] = &[&BURGLARS_PLOT];
