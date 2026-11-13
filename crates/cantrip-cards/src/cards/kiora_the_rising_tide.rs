use cantrip_api::*;
use cantrip_mechanics::{effects, tokens, triggers};
use cantrip_oracle as oracle;

const ASK: Question = Question::new("CreateToken", "create Scion of the Deep?", "A legendary 8/8 blue Octopus creature token", "Create Scion of the Deep", "Don't");

/// Scion of the Deep, a legendary 8/8 blue Octopus creature token (CR 205.4a, 111.4).
pub static SCION_OF_THE_DEEP: CardDef = CardDef {
    name: "Scion of the Deep",
    has_cost: false,
    colors: Some(ColorSet::of(Color::Blue)),
    types: TypeSet::of(&[CardType::Creature]),
    supertypes: Supertypes { basic: false, legendary: true, snow: false },
    subtypes: SubtypeSet::of(&[Subtype::Octopus]),
    power: Some(8),
    toughness: Some(8),
    ..CardDef::BLANK
};

/// "You may create Scion of the Deep": asked of the controller, then the token.
fn create_scion(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if effects::ask_may(g, item.controller, item.object, Purpose::Ask(&ASK))? {
        tokens::create(g, item.controller, &SCION_OF_THE_DEEP, 1)?;
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(triggers::THIS_ENTERS, effects::draw_two_then_discard_two, "When Kiora enters, draw two cards, then discard two cards."),
        triggered(
            triggers::THIS_ATTACKS,
            create_scion,
            "Threshold — Whenever Kiora attacks, if there are seven or more cards in your graveyard, you may create Scion of the Deep, a legendary 8/8 blue Octopus creature token.",
        )
        .condition(cantrip_mechanics::you_have_threshold),
    ],
    ..oracle::KIORA_THE_RISING_TIDE
};

pub static EXTRA: &[&CardDef] = &[&SCION_OF_THE_DEEP];
