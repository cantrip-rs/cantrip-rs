use cantrip_api::*;
use cantrip_mechanics::{library, tokens, triggers};
use cantrip_oracle as oracle;

/// Darkstar, the legendary 2/2 white and black Dog token Rufus Shinra makes (CR 111.1, 205.4a).
pub static DARKSTAR: CardDef = CardDef {
    name: "Darkstar",
    has_cost: false,
    colors: Some(ColorSet(0b00101)),
    types: TypeSet::of(&[CardType::Creature]),
    supertypes: Supertypes { basic: false, legendary: true, snow: false, world: false },
    subtypes: SubtypeSet::of(&[Subtype::Dog]),
    power: Some(2),
    toughness: Some(2),
    ..CardDef::BLANK
};

/// "If you don't control a creature named Darkstar" (CR 603.4): checked as the ability would trigger and again as it resolves,
/// off the board as it stands. A name is the one it has now (CR 201.2a), so a copy's or an effect's counts (CR 612.8); only
/// a creature you control counts, the opponent's Darkstar or a noncreature one does not.
fn dont_control_darkstar(q: &dyn Query, asker: &Asker) -> bool {
    !q.permanents_of(asker.controller).into_iter().any(|id| q.chars(id).is(CardType::Creature) && library::object_names(q, id).contains(&"Darkstar"))
}

/// "Create Darkstar, a legendary 2/2 white and black Dog creature token."
fn make_darkstar(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    tokens::create(g, item.controller, &DARKSTAR, 1)?;
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        triggers::THIS_ATTACKS,
        make_darkstar,
        "Whenever Rufus Shinra attacks, if you don't control a creature named Darkstar, create Darkstar, a legendary 2/2 white and black Dog creature token.",
    )
    .condition(dont_control_darkstar)],
    ..oracle::RUFUS_SHINRA
};

pub static EXTRA: &[&CardDef] = &[&DARKSTAR];
