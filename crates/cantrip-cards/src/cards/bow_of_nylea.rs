use cantrip_api::*;
use cantrip_mechanics::library::ask_library_order;
use cantrip_mechanics::{damage_target, effects, targets};
use cantrip_oracle as oracle;

/// One of "up to four target cards from your graveyard" (CR 115.1): a mode's target slots are one target each, so the
/// four are four optional slots, and a card an earlier one chose is not a target again (CR 115.3).
fn card_in_your_graveyard(q: &dyn Query, asker: &Asker, t: Target) -> bool {
    targets::card_in_your_graveyard(q, asker, t, &[]) && !q.chosen_targets().contains(&t)
}

const UP_TO_ONE_CARD: TargetSpec = up_to_one(card_in_your_graveyard);

/// Put a +1/+1 counter on target creature.
fn counter_on_creature(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    cantrip_mechanics::effects::plus_one_counter_on_target(g, item)
}

/// Bow of Nylea deals 2 damage to target creature with flying.
fn two_damage(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    damage_target(g, item, 0, 2);
    Ok(())
}

/// Put up to four target cards from your graveyard on the bottom of your library in any order (CR 401.4, 608.2b): the
/// targets still legal are ordered by the controller (asked only when there is more than one), moved together and
/// placed in that order.
fn bottom_in_any_order(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let you = item.controller;
    let cards = targets::legal_objects(g, item);
    if cards.is_empty() {
        return Ok(());
    }
    let order = ask_library_order(g, you, item.object, &cards, DestinationKind::LibraryBottom)?;
    g.put_into_library(&order, LibraryPlace::Bottom);
    Ok(())
}

static MODAL: ModalDef = modes(&[
    mode(counter_on_creature, "Put a +1/+1 counter on target creature.")
        .targets(&[targets::CREATURE]),
    mode(two_damage, "Bow of Nylea deals 2 damage to target creature with flying.")
        .targets(&[targets::CREATURE_WITH_FLYING]),
    mode(|g, item| effects::gain_life(g, item, 3), "You gain 3 life."),
    mode(bottom_in_any_order, "Put up to four target cards from your graveyard on the bottom of your library in any order.")
        .targets(&[UP_TO_ONE_CARD, UP_TO_ONE_CARD, UP_TO_ONE_CARD, UP_TO_ONE_CARD]),
]);

pub static CARD: CardDef = CardDef {
    abilities: &[modal_ability(cost("{1}{G}, {T}"), &MODAL, "{1}{G}, {T}: Choose one \u{2014} put a +1/+1 counter on target creature; Bow of Nylea deals 2 damage to target creature with flying; you gain 3 life; or put up to four target cards from your graveyard on the bottom of your library in any order.")],
    statics: &[static_ability(Layer::Ability, cantrip_mechanics::statics::attacking_creatures_you_control, cantrip_mechanics::statics::have_deathtouch, "Attacking creatures you control have deathtouch.")],
    ..oracle::BOW_OF_NYLEA
};
