//! Roaring Furnace // Steaming Sauna, a Room (CR 709.5): each door is only what its text does; the engine gives the Room
//! the abilities and play rules of its unlocked doors (`layers::doors`, `actions::each_play_rule`).

use cantrip_api::*;
use cantrip_mechanics::triggers::{self, YOUR_END_STEP};
use cantrip_mechanics::{cards_in_hand, damage_target, effects, targets};
use cantrip_oracle as oracle;

/// Roaring Furnace: "When you unlock this door, this Room deals damage equal to the number of cards in your hand to target
/// creature an opponent controls." The cards are counted as the ability resolves (CR 608.2h).
fn furnace(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let n = cards_in_hand(g, item.controller) as u16;
    damage_target(g, item, 0, n);
    Ok(())
}

pub static ROARING_FURNACE: CardDef = CardDef {
    abilities: &[triggered(
        triggers::YOU_UNLOCK_THIS_LEFT_DOOR,
        furnace,
        "When you unlock this door, this Room deals damage equal to the number of cards in your hand to target creature an opponent controls.",
    )
    .targets(&[targets::CREATURE_AN_OPPONENT_CONTROLS])],
    ..oracle::ROARING_FURNACE
};

/// Steaming Sauna: "You have no maximum hand size" (CR 402.2), while this door is unlocked. "At the beginning of your end
/// step, draw a card."
pub static STEAMING_SAUNA: CardDef = CardDef {
    abilities: &[triggered(YOUR_END_STEP, |g, item| effects::draw(g, item, 1), "At the beginning of your end step, draw a card.")],
    play_rules: &[PlayRule::MaximumHandSize(None)],
    ..oracle::STEAMING_SAUNA
};

pub static CARD: CardDef = CardDef { split: Some((&ROARING_FURNACE, &STEAMING_SAUNA)), ..oracle::ROARING_FURNACE_STEAMING_SAUNA };

pub static EXTRA: &[&CardDef] = &[&ROARING_FURNACE, &STEAMING_SAUNA];
