use cantrip_api::*;
use cantrip_mechanics::{history, lands_you_control, tokens, triggers};
use cantrip_oracle as oracle;

/// Primo, the Indivisible, the legendary 0/0 green and blue Fractal creature token Zimone makes (CR 111.1, 205.4a).
pub static PRIMO: CardDef = CardDef {
    name: "Primo, the Indivisible",
    has_cost: false,
    colors: Some(ColorSet(ColorSet::of(Color::Green).0 | ColorSet::of(Color::Blue).0)),
    types: TypeSet::of(&[CardType::Creature]),
    supertypes: Supertypes { basic: false, legendary: true, snow: false, world: false },
    subtypes: SubtypeSet::of(&[Subtype::Fractal]),
    power: Some(0),
    toughness: Some(0),
    ..CardDef::BLANK
};

/// "A prime number" (a whole number above 1 with no divisor but 1 and itself): any number of lands, not only the ones the
/// reminder text lists.
fn is_prime(n: usize) -> bool {
    n >= 2 && (2..n).all(|d| n % d != 0)
}

/// "If a land entered the battlefield under your control this turn and you control a prime number of lands" (CR 603.4): the
/// intervening "if", true as the ability would trigger and again as it resolves.
fn land_entered_and_prime_lands(q: &dyn Query, asker: &Asker) -> bool {
    history::land_entered_under_control_this_turn(q.history(), asker.controller) && is_prime(lands_you_control(q, asker.controller))
}

/// "Create Primo, the Indivisible ..., then put that many +1/+1 counters on it": as many counters as lands its controller
/// controls as the ability resolves (CR 608.2c), put on after the token has entered (it is 0/0 only until then), on every
/// Primo made (a token doubler makes two) as one event (CR 101.4).
fn create_primo_then_that_many_counters(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let lands = lands_you_control(g, item.controller) as i32;
    tokens::create_with_counters(g, item.controller, &PRIMO, lands)?;
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        triggers::YOUR_END_STEP,
        create_primo_then_that_many_counters,
        "At the beginning of your end step, if a land entered the battlefield under your control this turn and you control a prime number of lands, create Primo, the Indivisible, a legendary 0/0 green and blue Fractal creature token, then put that many +1/+1 counters on it.",
    )
    .condition(land_entered_and_prime_lands)],
    ..oracle::ZIMONE_ALL_QUESTIONING
};

pub static EXTRA: &[&CardDef] = &[&PRIMO];
