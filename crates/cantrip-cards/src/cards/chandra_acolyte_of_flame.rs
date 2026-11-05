use cantrip_api::*;
use cantrip_mechanics::{delayed, effects, targets, tokens};
use cantrip_oracle as oracle;

/// "0: Put a loyalty counter on each red planeswalker you control" (CR 306.5c, 122.1e): the planeswalkers its controller
/// controls as it resolves, Chandra herself among them, red as the words of its source's text read "red" (CR 612); all at once
/// (CR 608.2, 101.4). It activates no loyalty ability of the others (the rulings).
fn loyalty_on_red_planeswalkers(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let red = g.words(item.object).color(Color::Red);
    let walkers: Vec<(ObjectId, CounterKind, i64)> = g
        .permanents_of(item.controller)
        .into_iter()
        .filter(|&id| {
            let chars = g.chars(id);
            chars.is(CardType::Planeswalker) && chars.colors.has(red)
        })
        .map(|id| (id, CounterKind::Loyalty, 1))
        .collect();
    g.add_counters_together(&walkers);
    Ok(())
}

/// "0: Create two 1/1 red Elemental creature tokens. They gain haste. Sacrifice them at the beginning of the next end step"
/// (CR 111.1, 701.7a, 603.7c): the two enter together, gain haste with no duration (CR 611.2a), and a delayed ability about exactly
/// them sacrifices them.
fn elementals(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let made = tokens::create(g, item.controller, &tokens::ELEMENTAL_1_1_RED, 2)?;
    g.end_sentence();
    instruction(g, |g| {
        for &token in &made {
            effects::gain_haste(g, item.object, token);
        }
    });
    g.end_sentence();
    delayed::at_next_end_step_about(g, item, 0, &made);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        ability(loyalty(0), loyalty_on_red_planeswalkers, "0: Put a loyalty counter on each red planeswalker you control."),
        ability(loyalty(0), elementals, "0: Create two 1/1 red Elemental creature tokens. They gain haste. Sacrifice them at the beginning of the next end step."),
        ability(loyalty(-2), effects::may_cast_first_target_exiling_it_paying_costs, "\u{2212}2: You may cast target instant or sorcery card with mana value 3 or less from your graveyard. If that spell would be put into your graveyard, exile it instead.")
            .targets(&[targets::INSTANT_OR_SORCERY_CARD_WITH_MANA_VALUE_3_OR_LESS_IN_YOUR_GRAVEYARD]),
    ],
    delayed: &[delayed::SACRIFICE_CREATED],
    ..oracle::CHANDRA_ACOLYTE_OF_FLAME
};

pub static EXTRA: &[&CardDef] = &[&tokens::ELEMENTAL_1_1_RED];
