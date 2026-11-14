use cantrip_api::*;
use cantrip_mechanics::{choose, targets};
use cantrip_oracle as oracle;

/// "{R}, {T}, Discard a card: Put a +0/+1 counter or a +1/+0 counter on target creature." (CR 608.2, 122.1).
fn put_0_1_counter_1(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(target) = targets::object(g, item, 0) else { return Ok(()) };
    let card = item.ability.card;
    let chosen = choose::one_of(
        g, item.controller, item.object,
        vec![Label::Card(card, 0), Label::Card(card, 1)],
    )?;
    let counter = if chosen == 0 { CounterKind::Plus0Plus1 } else { CounterKind::Plus1Plus0 };
    g.add_counters(target, counter, 1);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        ability(
                cost("{R}").and(TAP).and(discard_cards(1)),
                put_0_1_counter_1,
                "{R}, {T}, Discard a card: Put a +0/+1 counter or a +1/+0 counter on target creature.",
            )
            .targets(&[targets::CREATURE]),
    ],
    labels: &["+0/+1 counter", "+1/+0 counter"],
    ..oracle::DWARVEN_ARMORER
};
