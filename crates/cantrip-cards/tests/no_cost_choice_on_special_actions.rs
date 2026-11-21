//! A special action (CR 116.2) is taken by `special_action`, which pays the ability's `cost` and never asks which of its
//! `cost_choice` ways is used: that is `choose_cost_way`, run for an activated ability, a mana ability included
//! (`activate_mana_as`, CR 605.3a), but not for a special action. One written with `pay_one_of` would therefore be taken
//! with that part of its cost unpaid. Until the core offers the choice there, no card may declare one.

use cantrip_api::*;

#[test]
fn no_special_action_has_a_choice_of_cost() {
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
            if a.special_action {
                wrong.push(format!("{}: \"{}\"", card.name, a.text));
            }
        }
    }
    assert!(wrong.is_empty(), "special actions whose choice of cost would never be asked for, so never paid: {wrong:?}");
    assert!(choices > 0, "no activated ability with a choice of cost was found: the test reads nothing");
}
