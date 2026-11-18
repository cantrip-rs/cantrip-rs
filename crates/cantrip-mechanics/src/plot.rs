//! Plot (CR 702.170a): "Any time you have priority during your main phase while the stack is empty, you may exile this card
//! from your hand and pay [cost]. It becomes a plotted card. Cast it as a sorcery on a later turn without paying its mana
//! cost." Pillage the Bog and Pitiless Carnage print it.

use cantrip_api::*;

/// The card, now in exile, becomes plotted (CR 702.170a): its owner may cast it without paying its mana cost, as a sorcery,
/// on a later turn (CR 702.170d), and `Event::Plotted` is emitted for "when this card becomes plotted". Nothing happens to a card
/// that is not in exile (a copy of a spell that was exiled ceases to exist). Plot's own special action, and Aven Interrupter's
/// "exile target spell. It becomes plotted", use it.
pub fn become_plotted(g: &mut dyn Game, exiled: ObjectId) {
    g.become_plotted(exiled);
}

/// "You may exile a nonland card with mana value 3 or less from your hand. If you do, it becomes plotted." (CR 702.170a) It is
/// a choice made as the effect resolves (CR 608.2d): declining is the empty answer to an up-to-one choice, and nothing is asked
/// of a hand with no such card. Jace Reawakened (+1) and Kellan Joins Up print it.
pub fn may_exile_a_cheap_nonland_card_from_hand_and_plot(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let you = item.controller;
    let hand = g.state().zone(Zone::hand(you)).to_vec();
    let eligible: Vec<ObjectId> = hand.iter().copied().filter(|&card| !g.chars(card).is(CardType::Land) && g.mana_value(card) <= 3).collect();
    let chosen = crate::choose::cards_from_kind(g, you, item.object, Purpose::Plot, "nonland", hand, eligible, 1, true)?;
    if let Some(&card) = chosen.first() {
        let exiled = g.exile(card);
        become_plotted(g, exiled);
    }
    Ok(())
}

/// Exile the card from the hand and let its owner cast it for free as a sorcery on a later turn (CR 702.170d).
fn plot_it(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let exiled = g.exile(item.object);
    become_plotted(g, exiled);
    Ok(())
}

/// Plot [cost]: a special action (CR 116.2k) from the hand, at sorcery timing. `text` is the ability's text, which a
/// scenario's `Ability:` matches.
pub const fn plot(cost: ManaCost, text: &'static str) -> AbilityDef {
    ability(Cost::mana(cost), plot_it, text)
        .sorcery_speed()
        .from(ZoneKind::Hand)
        .plot()
}
