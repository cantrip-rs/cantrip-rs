use cantrip_api::*;
use cantrip_mechanics::{library, targets};
use cantrip_oracle as oracle;

fn creature_or_planeswalker_you_control(q: &dyn Query, asker: &Asker, t: Target) -> bool {
    (targets::CREATURE_OR_PLANESWALKER.filter)(q, asker, t) && matches!(t, Target::Object(id) if q.controller(id) == asker.controller)
}

fn is_creature_or_planeswalker(q: &dyn Query, card: ObjectId) -> bool {
    let c = q.chars(card);
    c.is(CardType::Creature) || c.is(CardType::Planeswalker)
}

/// "Exile target creature or planeswalker you control. Reveal cards from the top of your library until you reveal a
/// creature or planeswalker card. Put that card onto the battlefield and the rest on the bottom of your library in a
/// random order." With the target illegal as the ability resolves none of it happens (CR 608.2b). The exile and the
/// reveal are separate steps (CR 608.2c): the card is found even when the permanent could not be exiled, and a library
/// holding no such card is revealed whole and put back in a random order (CR 701.20, 401.4).
fn exile_and_reveal(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let you = item.controller;
    let Some(target) = targets::object(g, item, 0) else { return Ok(()) };
    g.exile(target);
    let (found, rest) = library::reveal_until(g, you, 1, &is_creature_or_planeswalker);
    if let Some(card) = found.first().copied() {
        g.put_onto_battlefield(card, you, false)?;
    }
    g.bottom_in_random_order(you, &rest);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[ability(
        cost("{3}{R}"),
        exile_and_reveal,
        "{3}{R}: Exile target creature or planeswalker you control. Reveal cards from the top of your library until you reveal a creature or planeswalker card. Put that card onto the battlefield and the rest on the bottom of your library in a random order. Activate only as a sorcery.",
    )
    .targets(&[target(creature_or_planeswalker_you_control)])
    .sorcery_speed()],
    ..oracle::IDENTITY_ECHO
};
