use cantrip_api::*;
use cantrip_oracle as oracle;

/// "Return all instant and sorcery cards from your graveyard to your hand. You have no maximum hand size for the rest of the
/// game. Exile Wisdom of Ages." The cards are those in the controller's graveyard as it resolves, which does not hold the spell
/// itself, still on the stack, and they move together (CR 400.3: to their owner's hand, the graveyard's owner being its
/// controller here). The exile is the spell's last instruction.
fn effect(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let you = item.controller;
    let cards: Vec<ObjectId> = g
        .state()
        .zone(Zone::graveyard(you))
        .iter()
        .copied()
        .filter(|&card| g.chars(card).is(CardType::Instant) || g.chars(card).is(CardType::Sorcery))
        .collect();
    g.return_to_hand(&cards);
    g.end_sentence();
    g.add_effect(item.object, item.object, EffectKind::MaximumHandSize(None), Duration::Indefinite);
    g.end_sentence();
    g.exile(item.object);
    Ok(())
}

pub static CARD: CardDef = CardDef { spell: spell(&[], effect), ..oracle::WISDOM_OF_AGES };
