use cantrip_api::*;
use cantrip_mechanics::triggers::THIS_ENTERS;
use cantrip_mechanics::{class, effects, targets, tokens, cards_in_hand};
use cantrip_oracle as oracle;

/// Level 1: "each player creates a Food token" (CR 111.10b), together (CR 101.4).
fn each_player_creates_food(g: &mut dyn Game, _: &StackItem) -> Result<(), Suspend> {
    for player in g.state().apnap().collect::<Vec<_>>() {
        tokens::create_food(g, player)?;
    }
    Ok(())
}

/// Level 2: "Whenever an artifact an opponent controls is put into a graveyard from the battlefield" (CR 603.10a), looked back
/// on: the artifact as it last was there, under the controller it last had. Only once the Class has reached level 2.
fn opponents_artifact_goes_to_graveyard(q: &dyn Query, me: &Asker, event: &Event) -> Option<Fired> {
    match event {
        Event::ZoneChange { old, from, to, .. }
            if from.is(ZoneKind::Battlefield)
                && to.is(ZoneKind::Graveyard)
                && class::level(q, me.source) >= 2
                && q.chars(*old).is(CardType::Artifact)
                && q.obj(*old).controller != me.controller =>
        {
            Fired::PLAIN
        }
        _ => None,
    }
}

/// Level 3: "Whenever one or more of your opponents are attacked" (CR 508.1m): one declaration, with at least one creature
/// attacking an opponent of the controller directly, not a planeswalker or battle (CR 506.2). Only once the Class has reached
/// level 3.
fn opponents_are_attacked(q: &dyn Query, me: &Asker, event: &Event) -> Option<Fired> {
    match event {
        Event::AttackersDeclared { .. }
            if class::level(q, me.source) >= 3
                && q.state().attackers.iter().any(|a| matches!(a.target, Target::Player(p) if p != me.controller)) =>
        {
            Fired::PLAIN
        }
        _ => None,
    }
}

/// "Up to one target attacking creature gets +X/+X until end of turn, where X is the number of cards in your hand" (CR 611.2a):
/// counted as the ability resolves.
fn pump_by_hand(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(creature) = targets::object(g, item, 0) else { return Ok(()) };
    let x = i16::try_from(cards_in_hand(g, item.controller)).unwrap_or(i16::MAX);
    cantrip_mechanics::pump(g, item.object, creature, x, x);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        class::level_two("{1}{G}", "{1}{G}: Level 2"),
        class::level_three("{4}{G}", "{4}{G}: Level 3"),
        triggered(THIS_ENTERS, each_player_creates_food, "When this Class enters, each player creates a Food token."),
        triggered(
            when_asked(EventMask::ZONE_CHANGE, opponents_artifact_goes_to_graveyard),
            |g, item| effects::draw(g, item, 1),
            "Whenever an artifact an opponent controls is put into a graveyard from the battlefield, draw a card.",
        ),
        triggered(
            when_asked(EventMask::ATTACKERS_DECLARED, opponents_are_attacked),
            pump_by_hand,
            "Whenever one or more of your opponents are attacked, up to one target attacking creature gets +X/+X until end of turn, where X is the number of cards in your hand.",
        )
        .targets(&[targets::ATTACKING_CREATURE.optional()]),
    ],
    ..oracle::PARTY_DUDE
};

/// The Food token it makes.
pub static EXTRA: &[&CardDef] = &[&tokens::FOOD];
