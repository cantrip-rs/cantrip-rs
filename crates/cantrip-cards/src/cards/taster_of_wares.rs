use cantrip_api::*;
use cantrip_mechanics::{choose, permanents_with_subtype_you_control, select, targets, triggers};
use cantrip_oracle as oracle;

/// "Target opponent reveals X cards from their hand, where X is the number of Goblins you control. You choose one of those cards.
/// That player exiles it. If an instant or sorcery card is exiled this way, you may cast it for as long as you control this
/// creature, and mana of any type can be spent to cast that spell" (CR 701.20a, 608.2d, 607.2a): X is counted once, as the ability
/// resolves, among the Goblins its controller controls (this one included, if it is still there); the target opponent chooses
/// which cards to reveal, all of them when the hand holds no more than X; then the controller chooses one, and the opponent exiles
/// it, linked to this creature as it was when the trigger was put on the stack. An empty hand reveals nothing and nothing happens.
/// If the ability's controller no longer controls this creature as the card is exiled (an effect took it while the trigger was
/// on the stack), "for as long as you control this creature" (CR 611.2b, 113.8) has ended before it began: the card is exiled
/// with nothing linked to it, and nobody may cast it.
fn reveal_and_exile(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(opponent) = targets::player(g, item, 0) else { return Ok(()) };
    let hand = g.state().zone(Zone::hand(opponent)).to_vec();
    if hand.is_empty() {
        return Ok(());
    }
    let goblin = g.words(item.object).subtype(Subtype::Goblin);
    let count = cantrip_api::count_u32(permanents_with_subtype_you_control(g, item.controller, goblin));
    let revealed = choose::select(g, opponent, item.object, Purpose::Reveal, hand, Selection::new(select::as_many_as_can(count)))?;
    g.reveal(opponent, &revealed);
    let chosen = choose::select(g, item.controller, item.object, Purpose::Exile, revealed, Selection::new(select::exactly(1)))?;
    if g.is_on_battlefield(item.object) && g.controller(item.object) != item.controller {
        g.exile_all(&chosen);
        return Ok(());
    }
    let exiled = g.exile_with(&chosen, g.linked(item), false);
    // "If an instant or sorcery card is exiled this way, you may cast it for as long as you control this creature, and mana of
    // any type can be spent to cast that spell": an effect of the resolved ability (CR 611.2a, 609.4b) that ends for good when
    // control of this creature changes (CR 611.2b).
    for card in exiled {
        if g.chars(card).is(CardType::Instant) || g.chars(card).is(CardType::Sorcery) {
            g.allow_play(card, item.controller, PlayGrant { any_type: true, expiry: Expiry::WhileControls(item.object), ..PlayGrant::CAST });
        }
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        triggers::THIS_ENTERS,
        reveal_and_exile,
        "When this creature enters, target opponent reveals X cards from their hand, where X is the number of Goblins you control. You choose one of those cards. That player exiles it. If an instant or sorcery card is exiled this way, you may cast it for as long as you control this creature, and mana of any type can be spent to cast that spell.",
    )
    .targets(&[targets::OPPONENT])],
    ..oracle::TASTER_OF_WARES
};
