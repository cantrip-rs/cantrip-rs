//! Defiled Crypt // Cadaver Lab, a Room (CR 709.5): each door is only what its text does; the engine gives the Room the
//! abilities of its unlocked doors (`layers::doors`).

use cantrip_api::*;
use cantrip_mechanics::{effects, targets, tokens, triggers};
use cantrip_oracle as oracle;

/// The 2/2 black Horror enchantment creature token (CR 111.1, 111.4).
pub static HORROR: CardDef = CardDef {
    colors: Some(ColorSet::of(Color::Black)),
    types: TypeSet::of(&[CardType::Enchantment, CardType::Creature]),
    power: Some(2),
    toughness: Some(2),
    ..token!(Horror)
};

/// "Create a 2/2 black Horror enchantment creature token." (CR 701.7a, 111.2, 612.2a)
fn create_horror(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    tokens::create(g, item.controller, &HORROR, 1)?;
    Ok(())
}

/// Defiled Crypt: "Whenever one or more cards leave your graveyard, create a 2/2 black Horror enchantment creature token. This
/// ability triggers only once each turn." No more than one in a turn (`triggers_each_turn`), so the cards that leave together (CR 603.2c)
/// make one token.
pub static DEFILED_CRYPT: CardDef = CardDef {
    abilities: &[triggered(
        triggers::CARDS_LEAVE_YOUR_GRAVEYARD,
        create_horror,
        "Whenever one or more cards leave your graveyard, create a 2/2 black Horror enchantment creature token. This ability triggers only once each turn.",
    )
    .once_per_batch()
    .triggers_each_turn(1)],
    ..oracle::DEFILED_CRYPT
};

/// Cadaver Lab: "When you unlock this door, return target creature card from your graveyard to your hand."
pub static CADAVER_LAB: CardDef = CardDef {
    abilities: &[triggered(
        triggers::YOU_UNLOCK_THIS_RIGHT_DOOR,
        effects::return_first_target_to_hand,
        "When you unlock this door, return target creature card from your graveyard to your hand.",
    )
    .targets(&[targets::CREATURE_CARD_IN_YOUR_GRAVEYARD])],
    ..oracle::CADAVER_LAB
};

pub static CARD: CardDef = CardDef { split: Some((&DEFILED_CRYPT, &CADAVER_LAB)), ..oracle::DEFILED_CRYPT_CADAVER_LAB };

pub static EXTRA: &[&CardDef] = &[&DEFILED_CRYPT, &CADAVER_LAB, &HORROR];
