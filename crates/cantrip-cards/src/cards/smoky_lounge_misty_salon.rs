//! Smoky Lounge // Misty Salon, a Room (CR 709.5): each door is only what its text does; the engine gives the Room the
//! abilities of its unlocked doors (`layers::doors`).

use cantrip_api::*;
use cantrip_mechanics::triggers::{YOUR_FIRST_MAIN_PHASE, YOU_UNLOCK_THIS_RIGHT_DOOR};
use cantrip_mechanics::{rooms, tokens};
use cantrip_oracle as oracle;

/// "At the beginning of your first main phase, add {R}{R}. Spend this mana only to cast Room spells and unlock doors." (CR 106.4,
/// 106.6, 709.5e): each mana is restricted separately, and empties from the pool with the phase.
fn add_red_red(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    g.add_restricted_mana(item.controller, Mana::R, 2, ManaRestriction::Subtypes(SubtypeSpells::of(Subtype::Room).or_unlocking_doors()));
    Ok(())
}

/// "Create an X/X blue Spirit creature token with flying, where X is the number of unlocked doors among Rooms you control." X is
/// counted as the ability resolves (CR 608.2h, `rooms::unlocked_doors`: a Room with a door still locked counts one, CR 709.5c), so
/// this door, unlocked as the trigger was put on the stack, counts. The token is X/X as it is made (CR 111.4), never a 0/0 for a
/// moment, and is what this door's text says, a text change of it included (CR 612.4).
fn spirit_for_each_unlocked_door(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let x = rooms::unlocked_doors(g, item.object) as i32;
    tokens::create_sized(g, item.object, item.controller, &tokens::BLUE_FLYING_SPIRIT, (x, x), 1)?;
    Ok(())
}

/// Smoky Lounge.
pub static SMOKY_LOUNGE: CardDef = CardDef {
    abilities: &[triggered(
        YOUR_FIRST_MAIN_PHASE,
        add_red_red,
        "At the beginning of your first main phase, add {R}{R}. Spend this mana only to cast Room spells and unlock doors.",
    )
    .produces_mana()],
    ..oracle::SMOKY_LOUNGE
};

/// Misty Salon.
pub static MISTY_SALON: CardDef = CardDef {
    abilities: &[triggered(
        YOU_UNLOCK_THIS_RIGHT_DOOR,
        spirit_for_each_unlocked_door,
        "When you unlock this door, create an X/X blue Spirit creature token with flying, where X is the number of unlocked doors among Rooms you control.",
    )],
    ..oracle::MISTY_SALON
};

pub static CARD: CardDef = CardDef { split: Some((&SMOKY_LOUNGE, &MISTY_SALON)), ..oracle::SMOKY_LOUNGE_MISTY_SALON };

pub static EXTRA: &[&CardDef] = &[&SMOKY_LOUNGE, &MISTY_SALON, &tokens::BLUE_FLYING_SPIRIT];
