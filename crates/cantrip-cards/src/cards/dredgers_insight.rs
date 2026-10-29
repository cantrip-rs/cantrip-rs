use cantrip_api::*;
use cantrip_mechanics::{effects, library, triggers};
use cantrip_oracle as oracle;

/// "Whenever one or more artifact and/or creature cards leave your graveyard" (CR 603.2c, 603.10a): a card of the source's
/// controller's graveyard, as a card there, that is an artifact or a creature, going to any other zone (a token is in no
/// graveyard long enough to leave one, CR 111.7, 704.5d). The cards that left together are the subjects of the matching
/// events, so the batch triggers it once.
fn artifact_or_creature_cards_leave_your_graveyard(q: &dyn Query, me: &Asker, event: &Event) -> Option<Fired> {
    let fired = triggers::cards_leave_your_graveyard(q, me, event)?;
    let chars = q.chars(fired.subject.object()?);
    (chars.is(CardType::Artifact) || chars.is(CardType::Creature)).then_some(fired)
}

/// "Mill four cards. You may put an artifact, creature, or land card from among the milled cards into your hand."
fn mill_and_take(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    library::mill_then_may_take_into_hand(g, item.controller, item.object, 4, "artifact, creature, or land", |c| {
        c.is(CardType::Artifact) || c.is(CardType::Creature) || c.is(CardType::Land)
    })?;
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            when_asked(EventMask::ZONE_CHANGE, artifact_or_creature_cards_leave_your_graveyard),
            |g, item| effects::gain_life(g, item, 1),
            "Whenever one or more artifact and/or creature cards leave your graveyard, you gain 1 life.",
        )
        .once_per_batch(),
        triggered(triggers::THIS_ENTERS, mill_and_take, "When this enchantment enters, mill four cards. You may put an artifact, creature, or land card from among the milled cards into your hand."),
    ],
    ..oracle::DREDGERS_INSIGHT
};
