use cantrip_api::*;
use cantrip_mechanics::chosen_type::enters_choosing_elemental_elf_faerie_giant_goblin_kithkin_merfolk_or_treefolk;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

/// "Whenever a permanent you control of the chosen type enters" (CR 603.6a): any permanent, this artifact included, under the
/// controller's control as it entered, that is the type chosen for this ability (`Query::linked_to`) as it entered (CR 205.3).
/// With no type chosen (a copy that never entered) nothing is of it.
fn permanent_you_control_of_the_chosen_type_enters(q: &dyn Query, me: &Asker, event: &Event) -> Option<Fired> {
    match event {
        Event::ZoneChange { new, to, .. } if to.is(ZoneKind::Battlefield) && q.controller(*new) == me.controller => {
            let chosen = q.state().chosen_subtype(q.linked_to(me))?;
            if q.chars(*new).has_subtype(chosen) { Fired::PLAIN } else { None }
        }
        _ => None,
    }
}

/// "Target card of the chosen type from your graveyard" (CR 115.1, 205.3): a card in the ability controller's graveyard with the
/// type chosen for this ability, which it keeps as the source is sacrificed to pay the cost (CR 608.2h).
fn card_of_the_chosen_type_in_your_graveyard(q: &dyn Query, asker: &Asker, t: Target) -> bool {
    targets::any_card_in_your_graveyard(q, asker, t)
        && matches!(t, Target::Object(id) if q.state().chosen_subtype(q.linked_to(asker)).is_some_and(|chosen| q.chars(id).has_subtype(chosen)))
}

pub static CARD: CardDef = CardDef {
    enters: Some(enters_choosing_elemental_elf_faerie_giant_goblin_kithkin_merfolk_or_treefolk),
    abilities: &[
        triggered(
            when_asked(EventMask::ZONE_CHANGE, permanent_you_control_of_the_chosen_type_enters),
            |g, item| {
                g.gain_life(item.controller, 1);
                Ok(())
            },
            "Whenever a permanent you control of the chosen type enters, you gain 1 life.",
        ),
        ability(
            cost("{2}, {T}").and(SACRIFICE_THIS),
            effects::return_first_target_to_hand,
            "{2}, {T}, Sacrifice this artifact: Return target card of the chosen type from your graveyard to your hand.",
        )
        .targets(&[target(card_of_the_chosen_type_in_your_graveyard)]),
    ],
    ..oracle::DAWN_BLESSED_PENNANT
};
