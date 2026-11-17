use cantrip_api::*;
use cantrip_mechanics::{tokens, triggers};
use cantrip_oracle as oracle;

const PAY: Question = Question::new(
    "PayToReturn",
    "pay {cost} to return {subject} to the battlefield as a Treasure?",
    "It enters tapped under your control and loses all other card types",
    "Pay {cost}",
    "Don't pay",
);

/// "Whenever a nontoken creature an opponent controls dies" (CR 700.4, 603.10a, 111.7): a creature an opponent controlled as it died,
/// that was not a token, the ability about the card it became in the graveyard (CR 400.7).
fn nontoken_creature_an_opponent_controls_dies(q: &dyn Query, me: &Asker, event: &Event) -> Option<Fired> {
    let Event::ZoneChange { old, new, .. } = event else { return None };
    if q.obj(*old).token {
        return None;
    }
    triggers::creature_an_opponent_controls_dies(q, me, event)?;
    Fired::about(Target::Object(*new))
}

/// "You may pay {1}. If you do, return that card to the battlefield tapped under your control. It's a Treasure artifact with '{T},
/// Sacrifice this artifact: Add one mana of any color,' and it loses all other card types." (CR 118.12, 205.1a, 613.1d, 613.7): the card
/// returns only if it is still the card in the graveyard it died to (CR 400.7), and stays a Treasure artifact for as long as it is on the
/// battlefield (no duration, CR 611.2a), with Treasure its only artifact subtype; it keeps the abilities it printed, gains the Treasure's mana ability, and its creature types go
/// with its creature type (CR 205.3d).
fn pay_to_return_as_a_treasure(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Target::Object(card) = item.subject else { return Ok(()) };
    if !g.may_pay(item.controller, item.object, card, cost("{1}"), &PAY)? {
        return Ok(());
    }
    if g.obj(card).became.is_some() {
        return Ok(());
    }
    let treasure = g.put_onto_battlefield(card, item.controller, true)?;
    let source = g.card_id(&CARD);
    let forever = Duration::Indefinite;
    instruction(g, |g| {
        g.add_effect(item.object, treasure, EffectKind::SetTypes(TypeSet::of(&[CardType::Artifact])), forever);
        // "A Treasure artifact": Treasure is its only artifact subtype (CR 205.1a), so a Vehicle returns no longer a Vehicle.
        g.add_effect(item.object, treasure, EffectKind::LoseSubtypesOf(CardType::Artifact), forever);
        g.add_effect(item.object, treasure, EffectKind::AddSubtype(Subtype::Treasure), forever);
        g.add_effect(item.object, treasure, EffectKind::AddAbility(AbilityRef::delayed(source, 0)), forever);
    });
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        when_asked(EventMask::ZONE_CHANGE, nontoken_creature_an_opponent_controls_dies),
        pay_to_return_as_a_treasure,
        "Whenever a nontoken creature an opponent controls dies, you may pay {1}. If you do, return that card to the battlefield tapped under your control. It's a Treasure artifact with \"{T}, Sacrifice this artifact: Add one mana of any color,\" and it loses all other card types.",
    )],
    delayed: tokens::TREASURE.abilities,
    ..oracle::VRASKA_THE_SILENCER
};
