//! Paths no printed card in the engine reaches, played with every card the engine has (`cantrip_cards::all()`) beside test
//! cards made for each: one test binary, so the whole card table is linked once.

mod combat_counts;
mod concession;
mod counter_duration_ends;
mod demonic_pact;
mod eluge;
mod fantasticar_cant_be_sacrificed;
mod floodpits_drowner_self;
mod hybrid_reduction;
mod look_counts;
mod land_cant_be_sacrificed;
mod lost_abilities;
mod mana_ability_guards;
mod merged_components;
mod repeatable_mana;
mod righteous_valkyrie_noncreature;
mod search_any_number;
mod source_of_your_choice_delayed;
mod turn_face_up_by_effect;
mod until_exile;

/// Follow the explicit mana-ability window and then the actual mana payment; retain each test's preferred source.
fn pay(options: &[cantrip_engine::ActionOption], remaining: &cantrip_engine::ManaCost, mana_abilities: bool, preferred: Option<&cantrip_engine::ActionOption>) -> cantrip_engine::Answer {
    use cantrip_engine::{Action, ActionKind, Answer};
    let confirm = options.iter().find(|o| o.kind == ActionKind::ConfirmCasting);
    let assign = options.iter().find(|o| o.kind == ActionKind::AssignMana);
    let choice = if mana_abilities && !remaining.is_zero() { preferred.or(confirm) } else { assign.or(confirm) };
    Answer::Act(Action::of(choice.expect("the chosen payment can proceed")))
}
