use cantrip_api::*;
use cantrip_oracle as oracle;

/// "Whenever another nontoken artifact you control enters" (CR 603.6a): an artifact that is not a token, other than this
/// permanent, under your control as it entered. The entering artifact is the trigger's subject.
fn another_nontoken_artifact_you_control_enters(q: &dyn Query, me: &Asker, event: &Event) -> Option<Fired> {
    match event {
        Event::ZoneChange { new, to, .. }
            if *new != me.source
                && to.is(ZoneKind::Battlefield)
                && q.controller(*new) == me.controller
                && !q.obj(*new).token
                && q.chars(*new).is(CardType::Artifact) =>
        {
            Fired::about(Target::Object(*new))
        }
        _ => None,
    }
}

const PAY: Question = Question::new(
    "PayToCopy",
    "pay {cost} to create a token that's a copy of {subject}?",
    "If you do, it is a copy of the artifact, and a 2/2 Robot Villain creature in addition if it isn't a creature",
    "Pay",
    "Don't pay",
);

/// "You may pay {2}. If you do, create a token that's a copy of it. If the token isn't a creature, it becomes a 2/2 Robot
/// Villain creature in addition to its other types." The copy is of the artifact as it last existed (CR 707.2, 608.2h). What
/// the token becomes is no part of the copy (CR 707.9b is for the copy effect's exceptions): it is an effect on the token with
/// no duration (CR 611.2a), in layer 4 for the types and layer 7b for the power and toughness (CR 613.1d, 613.4b), so a copy of the
/// token is not a creature. It is judged as the token is, once it has entered.
fn copy(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Target::Object(artifact) = item.subject else { return Ok(()) };
    if !g.may_pay(item.controller, item.object, artifact, cost("{2}"), &PAY)? {
        return Ok(());
    }
    let values = g.copiable_values(artifact);
    let tokens = g.create_token_copies_of(values, item.controller, 1)?;
    g.one_instruction(&mut |g| {
        for &token in &tokens {
            if !is_live(g, token) || g.chars(token).is(CardType::Creature) {
                continue;
            }
            g.add_effect(item.object, token, EffectKind::AddTypes(TypeSet::of(&[CardType::Creature])), Duration::Indefinite);
            g.add_effect(item.object, token, EffectKind::AddSubtype(Subtype::Robot), Duration::Indefinite);
            g.add_effect(item.object, token, EffectKind::AddSubtype(Subtype::Villain), Duration::Indefinite);
            g.add_effect(item.object, token, EffectKind::SetPt(2, 2), Duration::Indefinite);
        }
        Ok(())
    })
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        when_asked(EventMask::ZONE_CHANGE, another_nontoken_artifact_you_control_enters),
        copy,
        "Whenever another nontoken artifact you control enters, you may pay {2}. If you do, create a token that's a copy of it. If the token isn't a creature, it becomes a 2/2 Robot Villain creature in addition to its other types.",
    )],
    ..oracle::ULTRON_ARTIFICIAL_MALEVOLENCE
};
