//! Bottomless Pool // Locker Room, a Room (CR 709.5): see `unholy_annex_ritual_chamber` for how a door is a half of the card.

use cantrip_api::*;
use cantrip_mechanics::triggers::{self, creature_you_control_deals_combat_damage_to_player};
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

/// Bottomless Pool: "When you unlock this door, return up to one target creature to its owner's hand." Unlocked as the Room
/// is cast or as a special action (CR 709.5e); with no target chosen nothing returns.
pub static BOTTOMLESS_POOL: CardDef = CardDef {
    abilities: &[triggered(
        triggers::YOU_UNLOCK_THIS_LEFT_DOOR,
        effects::return_first_target_to_hand,
        "When you unlock this door, return up to one target creature to its owner's hand.",
    )
    .targets(&[targets::CREATURE.between(0, 1)])],
    ..oracle::BOTTOMLESS_POOL
};

/// Locker Room: "Whenever one or more creatures you control deal combat damage to a player, draw a card." (CR 603.2c): one
/// trigger for each player the batch damaged, however many of the creatures dealt it.
pub static LOCKER_ROOM: CardDef = CardDef {
    abilities: &[triggered(
        when_asked(EventMask::DAMAGE, creature_you_control_deals_combat_damage_to_player),
        |g, item| effects::draw(g, item, 1),
        "Whenever one or more creatures you control deal combat damage to a player, draw a card.",
    )
    .sums_per_subject()],
    ..oracle::LOCKER_ROOM
};

pub static CARD: CardDef = CardDef { split: Some((&BOTTOMLESS_POOL, &LOCKER_ROOM)), ..oracle::BOTTOMLESS_POOL_LOCKER_ROOM };

pub static EXTRA: &[&CardDef] = &[&BOTTOMLESS_POOL, &LOCKER_ROOM];
