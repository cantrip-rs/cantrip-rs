//! Greenhouse // Rickety Gazebo, a Room (CR 709.5): each door is only what its text does; the Room has the abilities of
//! its unlocked doors (`layers::doors`, `CardTable::texts`).

use cantrip_api::*;
use cantrip_mechanics::{library, select, statics, triggers};
use cantrip_oracle as oracle;

/// Greenhouse: "Lands you control have '{T}: Add one mana of any color'" (CR 613.1f, layer 6), while this door is unlocked.
/// The ability granted is the door's own `delayed[0]` (`statics::grant_delayed_ability` reads the card whose static is being
/// applied), each land its own source (CR 113.6); the lands keep their other abilities.
pub static GREENHOUSE: CardDef = CardDef {
    delayed: &[mana_ability(TAP, add_any_color(1), "{T}: Add one mana of any color.")],
    statics: &[static_ability(
        Layer::Ability,
        statics::your_lands,
        statics::grant_delayed_ability,
        "Lands you control have \"{T}: Add one mana of any color.\"",
    )],
    ..oracle::GREENHOUSE
};

/// Rickety Gazebo: "When you unlock this door, mill four cards, then return up to two permanent cards from among them to
/// your hand" (CR 701.17a, 608.2d): the cards offered are those the mill moved, where they went.
fn mill_four_then_up_to_two_permanents_to_hand(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    library::mill_then_up_to_two_to_hand(g, item, 4, select::Selection::of(select::up_to(2), &[select::PERMANENT_CARD]).called("permanent"))
}

pub static RICKETY_GAZEBO: CardDef = CardDef {
    abilities: &[triggered(
        triggers::YOU_UNLOCK_THIS_RIGHT_DOOR,
        mill_four_then_up_to_two_permanents_to_hand,
        "When you unlock this door, mill four cards, then return up to two permanent cards from among them to your hand.",
    )],
    ..oracle::RICKETY_GAZEBO
};

pub static CARD: CardDef = CardDef { split: Some((&GREENHOUSE, &RICKETY_GAZEBO)), ..oracle::GREENHOUSE_RICKETY_GAZEBO };

pub static EXTRA: &[&CardDef] = &[&GREENHOUSE, &RICKETY_GAZEBO];
