use cantrip_api::*;
use cantrip_mechanics::statics::your_creatures;
use cantrip_mechanics::{tokens, triggers};
use cantrip_oracle as oracle;

pub static EXTRA: &[&CardDef] = &[&KITHKIN];

/// The 1/1 green and white Kithkin creature token (CR 111.4).
static KITHKIN: CardDef = CardDef {
    // White (bit 0) and green (bit 4).
    colors: Some(ColorSet(0b10001)),
    types: TypeSet::of(&[CardType::Creature]),
    power: Some(1),
    toughness: Some(1),
    ..token!(Kithkin)
};

/// "Creatures you control get +X/+X, where X is the number of creatures that entered the battlefield under your control this
/// turn" (CR 613.4c, layer 7c). X is asked of the event history (`Event::Entered`, CR 603.6a): each creature as it entered,
/// under the controller it entered under, whatever has happened to it since, so one that died or was exiled still counts and
/// one that entered under another player's control or was a noncreature does not.
fn plus_x_plus_x(view: &dyn LayerQuery, source: ObjectId, chars: &mut Chars) {
    let you = view.state().obj(source).controller;
    let x = view.history().entered().filter(|e| e.controller == you && e.is(CardType::Creature)).count() as i16;
    chars.power = chars.power.map(|v| v + x);
    chars.toughness = chars.toughness.map(|v| v + x);
}

/// "At the beginning of combat on your turn, create a 1/1 green and white Kithkin creature token."
fn make_kithkin(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    tokens::create(g, item.controller, &KITHKIN, 1)?;
    Ok(())
}

pub static CARD: CardDef = CardDef {
    statics: &[static_ability(
        Layer::PtModify,
        your_creatures,
        plus_x_plus_x,
        "Creatures you control get +X/+X, where X is the number of creatures that entered the battlefield under your control this turn.",
    )],
    abilities: &[triggered(
        triggers::BEGINNING_OF_YOUR_COMBAT,
        make_kithkin,
        "At the beginning of combat on your turn, create a 1/1 green and white Kithkin creature token.",
    )],
    ..oracle::KINBINDING
};
