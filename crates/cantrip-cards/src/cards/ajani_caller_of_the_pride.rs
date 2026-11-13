use cantrip_api::*;
use cantrip_mechanics::{effects, targets, tokens};
use cantrip_oracle as oracle;

/// The 2/2 white Cat creature token that the \u{2212}8 makes (CR 111.1).
pub static CAT: CardDef = CardDef {
    colors: Some(ColorSet::of(Color::White)),
    types: TypeSet::of(&[CardType::Creature]),
    power: Some(2),
    toughness: Some(2),
    ..token!(Cat)
};

/// \u{2212}8: "Create X 2/2 white Cat creature tokens, where X is your life total" (CR 111.1, 107.3a): X is your life total as the ability
/// resolves (CR 608.2h), made together as one event, none at 0 or less life.
fn create_cats_equal_to_your_life_total(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let life = g.state().player(item.controller).life.max(0) as u32;
    tokens::create(g, item.controller, &CAT, life)?;
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        ability(loyalty(1), effects::plus_one_counter_on_target, "+1: Put a +1/+1 counter on up to one target creature.")
            .targets(&[targets::CREATURE.optional()]),
        ability(
            loyalty(-3),
            effects::first_target_gains_until_end_of_turn::<{ Keywords::of(&[Keyword::Flying, Keyword::DoubleStrike]).0 }>,
            "\u{2212}3: Target creature gains flying and double strike until end of turn.",
        )
            .targets(&[targets::CREATURE]),
        ability(
            loyalty(-8),
            create_cats_equal_to_your_life_total,
            "\u{2212}8: Create X 2/2 white Cat creature tokens, where X is your life total.",
        ),
    ],
    ..oracle::AJANI_CALLER_OF_THE_PRIDE
};

pub static EXTRA: &[&CardDef] = &[&CAT];
