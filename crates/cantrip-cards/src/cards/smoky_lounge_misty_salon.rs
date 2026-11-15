//! Smoky Lounge // Misty Salon, a Room (CR 709.5): each door is only what its text does; the engine gives the Room the
//! abilities of its unlocked doors (`layers::doors`).

use cantrip_api::*;
use cantrip_mechanics::triggers::{YOUR_FIRST_MAIN_PHASE, YOU_UNLOCK_THIS_RIGHT_DOOR};
use cantrip_oracle as oracle;

/// The X/X blue Spirit creature token with flying Misty Salon makes: the generic definition, its size given as it is made.
pub static SPIRIT: CardDef = CardDef {
    colors: Some(ColorSet::of(Color::Blue)),
    types: TypeSet::of(&[CardType::Creature]),
    power: Some(0),
    toughness: Some(0),
    keywords: Keywords::of(&[Keyword::Flying]),
    text: "Flying",
    ..token!(Spirit)
};

/// "At the beginning of your first main phase, add {R}{R}. Spend this mana only to cast Room spells and unlock doors." (CR 106.4,
/// 106.6, 709.5e): each mana is restricted separately, and empties from the pool with the phase.
fn add_red_red(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    g.add_restricted_mana(item.controller, Mana::R, 2, ManaRestriction::Subtypes(SubtypeSpells::of(Subtype::Room).or_unlocking_doors()));
    Ok(())
}

/// "The number of unlocked doors among Rooms you control" (CR 709.5, 709.5c) as the effect reads it: a Room with both doors
/// unlocked counts two, one with a half still locked (CR 709.5b) counts one, and the subtype is read through the words of the
/// Room's text (CR 612).
fn unlocked_doors_among_your_rooms(q: &dyn Query, source: ObjectId) -> u32 {
    let you = q.controller(source);
    let room = q.words(source).subtype(Subtype::Room);
    q.state().battlefield().iter().filter(|&&id| q.controller(id) == you && q.chars(id).has_subtype(room)).map(|&id| q.obj(id).marks.doors.count_ones()).sum()
}

/// "Create an X/X blue Spirit creature token with flying, where X is the number of unlocked doors among Rooms you control." X is
/// counted as the ability resolves (CR 608.2h), so this door, unlocked as the trigger was put on the stack, counts. The token is
/// X/X as it is made (CR 111.4), never a 0/0 for a moment.
fn spirit_for_each_unlocked_door(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let x = unlocked_doors_among_your_rooms(g, item.object) as i32;
    let values = Copiable::of(g.card_id(&SPIRIT)).sized(x, x);
    g.create_token_copies_of(values, item.controller, 1)?;
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

pub static EXTRA: &[&CardDef] = &[&SMOKY_LOUNGE, &MISTY_SALON, &SPIRIT];
