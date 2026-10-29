use cantrip_api::*;
use cantrip_mechanics::effects::{plus_one_counter_on_this, scry};
use cantrip_mechanics::tokens;
use cantrip_mechanics::triggers::ANOTHER_CREATURE_YOU_CONTROL_ENTERS;
use cantrip_oracle as oracle;

/// "Choose one that hasn't been chosen this turn" (CR 700.2): whether mode `mode` has not been chosen this turn by the
/// object that has the ability (`asker.source`), read from the event history as each instance went on the stack. A mode chosen
/// for an ability that was then countered stays chosen, and a Lita that left and came back is a new object (CR 400.7) with every
/// mode to choose again; two of them each track their own (the rulings). When all three are chosen the ability is removed from
/// the stack with no effect (CR 603.3c), which the core does for a modal ability with too few modes to choose.
fn mode_not_chosen_this_turn(q: &dyn Query, asker: &Asker, mode: u8) -> bool {
    !q.history().this_turn().iter().any(|r| {
        matches!(r.event, Event::ModesChosen { source, modes, .. } if source == asker.source && modes & (1 << mode) != 0)
    })
}

static MODAL: ModalDef = modes(&[
    mode(|g, item| plus_one_counter_on_this(g, item), "Put a +1/+1 counter on Lita."),
    mode(|g, item| tokens::create_food(g, item.controller), "Create a Food token."),
    mode(|g, item| scry(g, item, 1), "Scry 1."),
])
    .allowed(mode_not_chosen_this_turn);

pub static CARD: CardDef = CardDef {
    abilities: &[modal_triggered(
        ANOTHER_CREATURE_YOU_CONTROL_ENTERS,
        &MODAL,
        "Alliance \u{2014} Whenever another creature you control enters, choose one that hasn't been chosen this turn \u{2014} Put a +1/+1 counter on Lita; or create a Food token; or scry 1.",
    )],
    ..oracle::LITA_LITTLE_ORPHAN_AMPHIBIAN
};

pub static EXTRA: &[&CardDef] = &[&tokens::FOOD];
