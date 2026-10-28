use cantrip_api::*;
use cantrip_mechanics::{empower, weaken_attackers};
use cantrip_oracle as oracle;

/// "+1: Empower Jace X, where X is the number of Islands you control" (CR 701.71a): X is counted as the ability resolves.
/// Island is read through the words of Jace's text (CR 612).
fn plus_one(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let island = g.words(item.object).subtype(Subtype::Island);
    let x = g.permanents_of(item.controller).into_iter().filter(|&id| g.chars(id).is(CardType::Land) && g.chars(id).has_subtype(island)).count() as i16;
    empower::jace(g, item.controller, item.object, x)
}

/// "-3: Until your next turn, whenever a creature attacks you or a planeswalker you control, it gets -5/-0 until end of turn."
fn minus_three(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    weaken_attackers::until_your_next_turn(g, item, &CARD);
    Ok(())
}

/// "0: Exile all but the bottom card of each opponent's library": the cards of every opponent's library go together (CR 101.4,
/// 701.13); the bottom card of each stays, and an empty library has none to keep.
fn zero(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let mut cards = Vec::new();
    for opponent in g.state().opponents(item.controller) {
        let library = g.state().zone(Zone::library(opponent));
        cards.extend(library.iter().copied().take(library.len().saturating_sub(1)));
    }
    g.exile_all(&cards);
    Ok(())
}

/// "Activate only if there are twenty-five or more loyalty counters among Jaces you control" (CR 602.5): every Jace
/// permanent the activating player controls, tokens and Jace itself included, is counted as the ability is activated.
fn twenty_five_loyalty_counters_among_your_jaces(q: &dyn Query, asker: &Asker) -> bool {
    let state = q.state();
    let counters: i32 = state
        .battlefield()
        .iter()
        .filter(|&&id| q.controller(id) == asker.controller && q.chars(id).has_subtype(Subtype::Jace))
        .map(|&id| i32::from(state.obj(id).counters(CounterKind::Loyalty)))
        .sum();
    counters >= 25
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        ability(loyalty(1), plus_one, "+1: Empower Jace X, where X is the number of Islands you control."),
        ability(loyalty(-3), minus_three, "\u{2212}3: Until your next turn, whenever a creature attacks you or a planeswalker you control, it gets -5/-0 until end of turn."),
        ability(loyalty(0), zero, "0: Exile all but the bottom card of each opponent's library. Activate only if there are twenty-five or more loyalty counters among Jaces you control.")
            .condition(twenty_five_loyalty_counters_among_your_jaces),
    ],
    delayed: &[weaken_attackers::delayed_ability::<5>("Whenever a creature attacks you or a planeswalker you control, it gets -5/-0 until end of turn.")],
    ..oracle::JACE_REALITY_SCULPTOR
};

pub static EXTRA: &[&CardDef] = &[&empower::JACE];
