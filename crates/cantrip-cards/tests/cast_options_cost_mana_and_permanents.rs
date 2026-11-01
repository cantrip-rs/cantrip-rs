//! The optional cost an ability gives a spell (`TriggeredDef::cast_option`: casualty, conspire, CR 601.2b) is paid by the
//! cast procedure as mana and permanents tapped or sacrificed, exiled or returned - the parts it announces, chooses and pays
//! for each such cost beside the printed one. A cost with any other part (life, a discard, counters) would be offered and
//! then left unpaid. This test holds every such ability to mana and permanents.

use cantrip_api::*;

#[test]
fn every_cast_option_costs_mana_and_permanents_only() {
    let table = CardTable::new(cantrip_cards::all());
    let mut found = 0;
    let mut wrong = Vec::new();
    for card in table.cards.iter() {
        for ability in card.abilities.iter().chain(card.delayed) {
            if let AbilityDef::Triggered(t) = ability
                && let Some(option) = t.cast_option
            {
                found += 1;
                if !option.cost.is_mana_and_permanents_only() {
                    wrong.push(card.name);
                }
            }
        }
    }
    assert!(wrong.is_empty(), "cast options paying more than mana and permanents: {wrong:?}");
    assert!(found > 0, "no cast option was found: the test reads nothing");
}
