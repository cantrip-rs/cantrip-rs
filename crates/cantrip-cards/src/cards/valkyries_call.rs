use cantrip_api::*;
use cantrip_oracle as oracle;

/// "Whenever a nontoken, non-Angel creature you control dies" (CR 700.4, 603.6c, 603.10a), looked back on: a creature as it last was
/// on the battlefield, under the controller it last had, so one that was an Angel there (even one that was a creature only by an
/// effect) is not one, and a token never is. The Angel type is read through the words of this enchantment's text (CR 612). The
/// trigger is about the card it became in the graveyard, which is what it returns.
fn nontoken_non_angel_creature_you_control_dies(q: &dyn Query, me: &Asker, event: &Event) -> Option<Fired> {
    match event {
        Event::ZoneChange { old, new, from, to, .. }
            if from.is(ZoneKind::Battlefield)
                && to.is(ZoneKind::Graveyard)
                && !q.obj(*old).token
                && q.chars(*old).is(CardType::Creature)
                && !q.chars(*old).has_subtype(q.words(me.source).subtype(Subtype::Angel))
                && q.obj(*old).controller == me.controller =>
        {
            Fired::about(Target::Object(*new))
        }
        _ => None,
    }
}

/// "It has flying and is an Angel in addition to its other types": what it returns as (`EntersAs`), from the moment it enters (CR
/// 614.12) and for as long as it is that object (CR 611.2a, 613.1d, 613.1f): a creature type gained and flying, every other type and
/// ability kept.
static AS_AN_ANGEL_WITH_FLYING: EntersAs = EntersAs { subtypes: &[Subtype::Angel], keywords: Keywords::of(&[Keyword::Flying]), ..EntersAs::NONE };

/// "Return that card to the battlefield under its owner's control with a +1/+1 counter on it" (CR 400.7, 110.2a, 122.1): it enters
/// under its owner's, not its last controller's, control with the counter (CR 614.1c). A card that has left the graveyard since the
/// trigger went on the stack is another object and returns nothing.
fn return_as_an_angel(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Target::Object(card) = item.subject else { return Ok(()) };
    let owner = g.obj(card).owner;
    g.put_all_onto_battlefield_becoming(&[(card, owner)], item, &AS_AN_ANGEL_WITH_FLYING, &[(CounterKind::PlusOne, 1)])?;
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        when_asked(EventMask::ZONE_CHANGE, nontoken_non_angel_creature_you_control_dies),
        return_as_an_angel,
        "Whenever a nontoken, non-Angel creature you control dies, return that card to the battlefield under its owner's control with a +1/+1 counter on it. It has flying and is an Angel in addition to its other types.",
    )],
    ..oracle::VALKYRIES_CALL
};
