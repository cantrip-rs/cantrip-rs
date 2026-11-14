//! Derelict Attic // Widow's Walk, a Room (CR 709.5): each door is only what its text does; the engine gives the Room the
//! abilities of its unlocked doors (`layers::doors`).

use cantrip_api::*;
use cantrip_mechanics::effects::{self, draw_two_cards_and_lose_2_life};
use cantrip_mechanics::triggers;
use cantrip_oracle as oracle;

/// Derelict Attic: "When you unlock this door, you draw two cards and you lose 2 life."
pub static DERELICT_ATTIC: CardDef = CardDef {
    abilities: &[triggered(
        triggers::YOU_UNLOCK_THIS_LEFT_DOOR,
        draw_two_cards_and_lose_2_life,
        "When you unlock this door, you draw two cards and you lose 2 life.",
    )],
    ..oracle::DERELICT_ATTIC
};

/// "It gets +1/+0 and gains deathtouch until end of turn" (CR 611.2a, 613.4c, 702.2): the creature that attacked alone, the
/// trigger's subject, whichever creature of its controller's it is, as one instruction (CR 608.2c); nothing once it has left
/// the battlefield (CR 400.7).
fn pump_and_gain_deathtouch(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Target::Object(creature) = item.subject else { return Ok(()) };
    instruction(g, |g| {
        cantrip_mechanics::pump(g, item.object, creature, 1, 0);
        if g.is_on_battlefield(creature) {
            effects::gain_until_end_of_turn(g, item.object, creature, Keyword::Deathtouch);
        }
    });
    Ok(())
}

/// Widow's Walk: "Whenever a creature you control attacks alone, it gets +1/+0 and gains deathtouch until end of turn."
pub static WIDOWS_WALK: CardDef = CardDef {
    abilities: &[triggered(
        triggers::CREATURE_YOU_CONTROL_ATTACKS_ALONE,
        pump_and_gain_deathtouch,
        "Whenever a creature you control attacks alone, it gets +1/+0 and gains deathtouch until end of turn.",
    )],
    ..oracle::WIDOWS_WALK
};

pub static CARD: CardDef = CardDef { split: Some((&DERELICT_ATTIC, &WIDOWS_WALK)), ..oracle::DERELICT_ATTIC_WIDOWS_WALK };

pub static EXTRA: &[&CardDef] = &[&DERELICT_ATTIC, &WIDOWS_WALK];
