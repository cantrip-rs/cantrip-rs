//! Necropolis's X comes from one physical card in exile, where X in its cost is zero (CR 202.3e).
//! Every registered face, including combined split cards, must fit the existing i32 counter API.
use cantrip_api::{CardTable, ManaCost};

fn off_stack_value(cost: ManaCost) -> u64 {
    // u64 keeps this check meaningful even if future card data exceeds u32 or i32 when summed.
    u64::from(cost.generic)
        + u64::from(cost.snow)
        + cost.symbols.iter().map(|&n| u64::from(n)).sum::<u64>()
        + cost.hybrid.iter().flatten().count() as u64
        + 2 * cost.twobrid.iter().map(|&n| u64::from(n)).sum::<u64>()
        + cost.phyrexian.iter().map(|&n| u64::from(n)).sum::<u64>()
}

#[test]
fn registered_off_stack_card_mana_values_fit_necropolis_counter_amount() {
    let table = CardTable::new(cantrip_cards::all());
    assert!(!table.cards.is_empty());
    // The table includes each split half as well as its combined off-stack face (CR 709.4).
    for name in ["Bedeck", "Bedazzle", "Bedeck // Bedazzle"] {
        assert!(table.cards.iter().any(|card| card.name == name), "missing registered split face {name}");
    }
    let combined = table.cards.iter().find(|card| card.name == "Bedeck // Bedazzle").unwrap();
    assert_eq!(off_stack_value(combined.cost), 8);
    for card in &table.cards {
        let value = off_stack_value(card.cost);
        assert!(value <= i32::MAX as u64, "{} has off-stack mana value {value}, outside Necropolis's counter API", card.name);
    }
}

#[test]
fn bound_check_detects_an_unrepresentable_combined_cost_without_overflowing() {
    let mut cost = ManaCost::ZERO;
    cost.generic = i32::MAX as u32;
    cost.symbols[0] = 1;
    assert!(off_stack_value(cost) > i32::MAX as u64);
}
