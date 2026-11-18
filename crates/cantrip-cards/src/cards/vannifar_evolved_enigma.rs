use cantrip_api::*;
use cantrip_mechanics::triggers::BEGINNING_OF_YOUR_COMBAT;
use cantrip_mechanics::{choose, cloak, effects, select};
use cantrip_oracle as oracle;

/// "Cloak a card from your hand" (CR 701.58a): its controller chooses a card in their hand as the mode resolves and puts it onto the
/// battlefield face down as a 2/2 creature with ward {2}, one they control. With an empty hand there is nothing to choose or cloak.
fn cloak_a_card_from_your_hand(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let player = item.controller;
    let hand = g.state().zone(Zone::hand(player)).to_vec();
    let chosen = choose::select(g, player, item.object, Purpose::Choose, hand, select::cards(select::exactly(1)))?;
    cloak::cloak(g, player, &chosen, false)?;
    Ok(())
}

/// "Put a +1/+1 counter on each colorless creature you control" (CR 122.1, 608.2): the creatures it controls as the mode resolves, judged
/// by the colors they have then - a face-down permanent has none (CR 708.2) - all at once (CR 101.4).
fn counter_on_each_colorless_creature(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let colorless: Vec<ObjectId> = g
        .permanents_of(item.controller)
        .into_iter()
        .filter(|&id| g.chars(id).is(CardType::Creature) && g.chars(id).colors.is_empty())
        .collect();
    effects::counters_on_each(g, &colorless, CounterKind::PlusOne, 1);
    Ok(())
}

static MODAL: ModalDef = modes(&[
    mode(cloak_a_card_from_your_hand, "Cloak a card from your hand."),
    mode(counter_on_each_colorless_creature, "Put a +1/+1 counter on each colorless creature you control."),
]);

pub static CARD: CardDef = CardDef {
    abilities: &[modal_triggered(
        BEGINNING_OF_YOUR_COMBAT,
        &MODAL,
        "At the beginning of combat on your turn, choose one \u{2014} Cloak a card from your hand; or put a +1/+1 counter on each colorless creature you control.",
    )],
    ..oracle::VANNIFAR_EVOLVED_ENIGMA
};
