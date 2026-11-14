use cantrip_api::*;
use cantrip_mechanics::triggers::{THIS_ATTACKS, THIS_ENTERS};
use cantrip_mechanics::{creatures_you_control, pump, targets, tokens};
use cantrip_oracle as oracle;

/// Angelo, the legendary 1/1 green and white Dog token Rinoa makes (CR 111.1, 205.4a).
pub static ANGELO: CardDef = CardDef {
    name: "Angelo",
    has_cost: false,
    colors: Some(ColorSet(0b10001)),
    types: TypeSet::of(&[CardType::Creature]),
    supertypes: Supertypes { basic: false, legendary: true, snow: false, world: false },
    subtypes: SubtypeSet::of(&[Subtype::Dog]),
    power: Some(1),
    toughness: Some(1),
    ..CardDef::BLANK
};

/// "When Rinoa Heartilly enters, create Angelo, a legendary 1/1 green and white Dog creature token."
fn make_angelo(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    tokens::create(g, item.controller, &ANGELO, 1)?;
    Ok(())
}

/// "Another target creature you control gets +1/+1 until end of turn for each creature you control" (CR 611.2a, 611.2c): the
/// creatures are counted once, as the ability resolves, the target and Rinoa included, and the bonus is fixed by that count
/// (CR 608.2h). Nothing if the target has left the battlefield or is no longer legal (CR 608.2b).
fn angelo_cannon(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(creature) = targets::object(g, item, 0) else { return Ok(()) };
    let count = creatures_you_control(g, item.controller) as i32;
    pump(g, item.object, creature, count, count);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(THIS_ENTERS, make_angelo, "When Rinoa Heartilly enters, create Angelo, a legendary 1/1 green and white Dog creature token."),
        triggered(
            THIS_ATTACKS,
            angelo_cannon,
            "Angelo Cannon \u{2014} Whenever Rinoa Heartilly attacks, another target creature you control gets +1/+1 until end of turn for each creature you control.",
        )
        .targets(&[targets::OTHER_CREATURE_YOU_CONTROL]),
    ],
    ..oracle::RINOA_HEARTILLY
};

pub static EXTRA: &[&CardDef] = &[&ANGELO];
