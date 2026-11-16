//! A mana ability is activated by `activate_mana_as` (CR 605.3a), which pays the ability's `cost` and never asks which of its
//! `cost_choice` ways is used: that is `choose_cost_way`, run only for an activated ability that uses the stack. A mana ability
//! written with `pay_one_of` ("{T}, Sacrifice a Food or exile three cards from your graveyard: Add ...") would therefore be
//! activated with that part of its cost unpaid. Until the core offers the choice there, no card may declare one.

use cantrip_api::*;

#[test]
fn no_mana_ability_has_a_choice_of_cost() {
    let table = CardTable::new(cantrip_cards::all());
    let mut wrong = Vec::new();
    let mut choices = 0;
    for card in table.cards.iter() {
        for ability in card.abilities {
            let AbilityDef::Activated(a) = ability else { continue };
            if a.cost_choice.is_empty() {
                continue;
            }
            choices += 1;
            if a.is_mana_ability() {
                wrong.push(format!("{}: \"{}\"", card.name, a.text));
            }
        }
    }
    assert!(wrong.is_empty(), "mana abilities whose choice of cost activate_mana_as would never ask for, so never pay: {wrong:?}");
    assert!(choices > 0, "no activated ability with a choice of cost was found: the test reads nothing");
}
