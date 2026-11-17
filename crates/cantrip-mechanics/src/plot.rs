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
