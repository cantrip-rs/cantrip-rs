//! Painter's Studio // Defaced Gallery, a Room (CR 709.5): each door is only what its text does; the engine gives the Room the
//! abilities of its unlocked doors (`layers::doors`).

use cantrip_api::*;
use cantrip_mechanics::triggers::{YOU_ATTACK, YOU_UNLOCK_THIS_LEFT_DOOR};
use cantrip_mechanics::{effects, library};
use cantrip_oracle as oracle;

fn exile_top_two_play_until_end_of_next_turn(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    library::exile_top_play_until_end_of_next_turn(g, item.controller, 2);
    Ok(())
}

/// Painter's Studio: "When you unlock this door, exile the top two cards of your library. You may play them until the end of your
/// next turn."
pub static PAINTERS_STUDIO: CardDef = CardDef {
    abilities: &[triggered(
        YOU_UNLOCK_THIS_LEFT_DOOR,
        exile_top_two_play_until_end_of_next_turn,
        "When you unlock this door, exile the top two cards of your library. You may play them until the end of your next turn.",
    )],
    ..oracle::PAINTERS_STUDIO
};

/// Defaced Gallery: "Whenever you attack, attacking creatures you control get +1/+0 until end of turn."
pub static DEFACED_GALLERY: CardDef = CardDef {
    abilities: &[triggered(
        YOU_ATTACK,
        effects::attacking_creatures_you_control_get::<1, 0>,
        "Whenever you attack, attacking creatures you control get +1/+0 until end of turn.",
    )],
    ..oracle::DEFACED_GALLERY
};

pub static CARD: CardDef = CardDef { split: Some((&PAINTERS_STUDIO, &DEFACED_GALLERY)), ..oracle::PAINTERS_STUDIO_DEFACED_GALLERY };

pub static EXTRA: &[&CardDef] = &[&PAINTERS_STUDIO, &DEFACED_GALLERY];
