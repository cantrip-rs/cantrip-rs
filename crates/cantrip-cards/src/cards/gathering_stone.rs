use cantrip_api::*;
use cantrip_mechanics::chosen_type::enters_choosing_a_creature_type;
use cantrip_mechanics::effects::ask_may;
use cantrip_mechanics::top_of_library;
use cantrip_mechanics::triggers::THIS_ENTERS_OR_YOUR_UPKEEP;
use cantrip_oracle as oracle;

const REVEAL: Question = Question::new(
    "RevealChosenTypeToHand",
    "reveal it and put it into your hand?",
    "It is a card of the chosen type. If you don't, you may put it into your graveyard instead",
    "Reveal it",
    "Don't",
);

const BIN: Question = Question::new(
    "PutTopIntoGraveyard",
    "put it into your graveyard?",
    "The card you looked at stays on top of your library if you don't",
    "Put it into your graveyard",
    "Leave it on top",
);

/// "Spells you cast of the chosen type cost {1} less to cast" (CR 601.2f, 205.3): only its controller's spells, judged on the
/// spell's own subtypes as it is cast; a generic reduction takes only the generic part (CR 118.7a, the 2025-11-17 ruling).
/// With no type chosen (a copy that never entered) nothing is of it.
fn chosen_type_spells_cost_less(q: &dyn Query, source: ObjectId, player: PlayerId, costed: Costed, _: &[Target]) -> CostDelta {
    let Costed::Spell(spell) = costed else { return CostDelta::NONE };
    if player != q.controller(source) {
        return CostDelta::NONE;
    }
    match q.state().chosen_subtype(q.text_linked(source)) {
        Some(chosen) if q.chars(spell).has_subtype(chosen) => CostDelta::less(1),
        _ => CostDelta::NONE,
    }
}

/// "Look at the top card of your library. If it's a card of the chosen type, you may reveal it and put it into your hand. If
/// you don't put the card into your hand, you may put it into your graveyard." (CR 701.20a.) The card is looked at whatever it
/// is, and may be left unrevealed even if it is of the chosen type (the 2025-11-17 ruling); not put into the hand, it may go to
/// the graveyard - no discard, no mill. An empty library has nothing to look at.
fn look_at_the_top_card(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let you = item.controller;
    let Some(&top) = top_of_library(g, you, 1).first() else {
        return Ok(());
    };
    g.look_at(you, &[top]);
    let asker = g.item_asker(item, you);
    let of_the_chosen_type = g.state().chosen_subtype(g.linked_to(&asker)).is_some_and(|chosen| g.chars(top).has_subtype(chosen));
    if of_the_chosen_type && ask_may(g, you, item.object, Purpose::Ask(&REVEAL))? {
        g.reveal(you, &[top]);
        g.return_to_hand_revealed(&[top]);
        return Ok(());
    }
    if ask_may(g, you, item.object, Purpose::Ask(&BIN))? {
        g.put_into_graveyard(&[top]);
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    enters: Some(enters_choosing_a_creature_type),
    cost_changes: &[chosen_type_spells_cost_less],
    abilities: &[triggered(
        THIS_ENTERS_OR_YOUR_UPKEEP,
        look_at_the_top_card,
        "When this artifact enters and at the beginning of your upkeep, look at the top card of your library. If it's a card of the chosen type, you may reveal it and put it into your hand. If you don't put the card into your hand, you may put it into your graveyard.",
    )],
    ..oracle::GATHERING_STONE
};
