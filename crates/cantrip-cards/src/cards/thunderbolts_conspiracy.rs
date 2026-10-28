use cantrip_api::*;
use cantrip_oracle as oracle;

/// "Whenever a Villain you control dies" (CR 700.4, 603.6c, 603.10a), looked back on: a creature that was a Villain as it last
/// was on the battlefield, under the controller it last had, its creature type read through the words of this enchantment's
/// text (CR 612). The trigger is about the card it became in the graveyard, which is what it returns; a token has no card to return.
fn villain_you_control_dies(q: &dyn Query, me: &Asker, event: &Event) -> Option<Fired> {
    match event {
        Event::ZoneChange { old, new, from, to, .. }
            if from.is(ZoneKind::Battlefield)
                && to.is(ZoneKind::Graveyard)
                && q.obj(*old).controller == me.controller
                && q.chars(*old).is(CardType::Creature)
                && q.chars(*old).has_subtype(q.words(me.source).subtype(Subtype::Villain)) =>
        {
            Fired::about(Target::Object(*new))
        }
        _ => None,
    }
}

/// "Return it to the battlefield under its owner's control with a finality counter on it. That creature is a Hero in addition to
/// its other types." (CR 400.7, 122.1h, 205.1b, 613.1d): the card is a new object as it returns, entering with the counter
/// (CR 614.1c); the type is a permanent layer-4 effect on that new object, which lasts as long as it is on the battlefield.
/// The card changed zones before this resolves (exiled in response, say) and nothing returns; a token has ceased to exist.
fn return_as_a_hero(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Target::Object(card) = item.subject else { return Ok(()) };
    let owner = g.obj(card).owner;
    let returned = g.put_onto_battlefield_with_counters(card, owner, false, &[(CounterKind::Finality, 1)])?;
    g.end_sentence();
    if g.is_on_battlefield(returned) {
        let hero = g.words(item.object).subtype(Subtype::Hero);
        g.add_effect(item.object, returned, EffectKind::AddSubtype(hero), Duration::Indefinite);
    }
    Ok(())
}

/// Flash is a core keyword read from its characteristics.
pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        when_asked(EventMask::ZONE_CHANGE, villain_you_control_dies),
        return_as_a_hero,
        "Whenever a Villain you control dies, return it to the battlefield under its owner's control with a finality counter on it. That creature is a Hero in addition to its other types.",
    )],
    ..oracle::THUNDERBOLTS_CONSPIRACY
};
