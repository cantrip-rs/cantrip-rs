//! Unholy Annex // Ritual Chamber, a Room (CR 709.5): a permanent with two doors, each a half of the card. Cast one half
//! and only that door starts unlocked; the other is unlocked by paying its mana cost as a special action (CR 709.5e).
//! The engine gives the Room its name, mana cost and abilities by its unlocked doors (`layers::doors`); each door here is
//! only what its text does. Other Rooms this makes possible are every one in Duskmourn: Bottomless Pool // Locker Room,
//! Crude Abattoir // Unsavory Kitchen, Funeral Room // Awakening Hall, Derelict Attic // Widow's Walk and the rest.

use cantrip_api::*;
use cantrip_mechanics::triggers::{self, YOUR_END_STEP};
use cantrip_oracle as oracle;

/// The 6/6 black Demon creature token with flying.
pub static DEMON: CardDef = CardDef {
    colors: Some(ColorSet::of(Color::Black)),
    types: TypeSet::of(&[CardType::Creature]),
    power: Some(6),
    toughness: Some(6),
    keywords: Keywords::of(&[Keyword::Flying]),
    text: "Flying",
    ..token!(Demon)
};

/// Unholy Annex: "At the beginning of your end step, draw a card. If you control a Demon, each opponent loses 2 life and
/// you gain 2 life. Otherwise, you lose 2 life." Whether you control a Demon is asked as the ability resolves, after the
/// draw, by the Demon type as this Room's text has it (CR 612).
fn annex(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let you = item.controller;
    g.draw(you, 1);
    let demon = g.words(item.object).subtype(Subtype::Demon);
    if g.permanents_of(you).into_iter().any(|id| g.chars(id).has_subtype(demon)) {
        cantrip_mechanics::effects::each_opponent_loses_life_you_gain_life::<2>(g, item)?;
    } else {
        g.lose_life(you, 2);
    }
    Ok(())
}

/// Ritual Chamber: "When you unlock this door, create a 6/6 black Demon creature token with flying."
fn chamber(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    cantrip_mechanics::tokens::create(g, item.controller, &DEMON, 1)?;
    Ok(())
}

pub static UNHOLY_ANNEX: CardDef = CardDef {
    abilities: &[triggered(YOUR_END_STEP, annex, "At the beginning of your end step, draw a card. If you control a Demon, each opponent loses 2 life and you gain 2 life. Otherwise, you lose 2 life.")],
    ..oracle::UNHOLY_ANNEX
};

pub static RITUAL_CHAMBER: CardDef = CardDef {
    abilities: &[triggered(triggers::YOU_UNLOCK_THIS_RIGHT_DOOR, chamber, "When you unlock this door, create a 6/6 black Demon creature token with flying.")],
    ..oracle::RITUAL_CHAMBER
};

pub static CARD: CardDef = CardDef { split: Some((&UNHOLY_ANNEX, &RITUAL_CHAMBER)), ..oracle::UNHOLY_ANNEX_RITUAL_CHAMBER };

pub static EXTRA: &[&CardDef] = &[&UNHOLY_ANNEX, &RITUAL_CHAMBER, &DEMON];
