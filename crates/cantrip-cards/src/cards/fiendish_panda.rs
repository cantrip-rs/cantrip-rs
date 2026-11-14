use cantrip_api::*;
use cantrip_mechanics::{effects, targets, triggers};
use cantrip_oracle as oracle;

/// "Another target non-Bear creature card with mana value less than or equal to this creature's power from your graveyard"
/// (CR 115.1, 205.3m, 113.7a): the Panda has died, so the card it became is in that graveyard, a new object (CR 400.7) excluded
/// by "another" - every card of it, if it was a merged permanent (CR 730.3c), and whatever card it is (a Clone that copied it is
/// no Bear there); its power is the last it had on the battlefield (`power_or_last_known`, counters included).
fn another_non_bear_creature_card_up_to_its_power(q: &dyn Query, asker: &Asker, t: Target) -> bool {
    targets::creature_card_in_your_graveyard_up_to_source_power(q, asker, t)
        && matches!(t, Target::Object(id) if !q.found_all_after_move(asker.source).contains(&id) && !q.chars(id).has_subtype(q.words(asker.source).subtype(Subtype::Bear)))
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(triggers::YOU_GAIN_LIFE, effects::plus_one_counter_on_this, "Whenever you gain life, put a +1/+1 counter on this creature."),
        triggered(
            triggers::THIS_DIES,
            effects::return_first_target_to_battlefield,
            "When this creature dies, return another target non-Bear creature card with mana value less than or equal to this creature's power from your graveyard to the battlefield.",
        )
        .targets(&[target(another_non_bear_creature_card_up_to_its_power)]),
    ],
    ..oracle::FIENDISH_PANDA
};
