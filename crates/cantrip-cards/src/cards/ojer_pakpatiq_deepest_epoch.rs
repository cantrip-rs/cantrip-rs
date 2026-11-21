use cantrip_api::*;
use cantrip_mechanics::triggers::you_cast_an_instant_or_sorcery_from_your_hand;
use cantrip_mechanics::effects;
use cantrip_oracle as oracle;

/// "Whenever you cast an instant spell from your hand" (CR 601.2a, 603.2, 109.5): a spell cast, not copied (CR 707.10), by this
/// ability's controller, from a hand that is theirs (a hand belongs to the owner of the cards in it, CR 400.3), that is an
/// instant as it stands on the stack. The spell rides along as the trigger's subject ("it").
fn you_cast_an_instant_from_your_hand(q: &dyn Query, me: &Asker, event: &Event) -> Option<Fired> {
    let Event::SpellCast { spell, .. } = event else { return None };
    if !q.chars(*spell).is(CardType::Instant) || q.obj(*spell).owner != me.controller {
        return None;
    }
    you_cast_an_instant_or_sorcery_from_your_hand(q, me, event)
}

/// "It gains rebound" (CR 702.88a, 613.1f): the spell that was cast has rebound while it is on the stack, so as it resolves it
/// is exiled instead of put into its owner's graveyard and may be cast again at the beginning of its controller's next upkeep.
/// It was cast from its controller's hand, as the trigger asked, so rebound's own condition holds. A spell already gone (the
/// trigger resolves first, so it is gone only if something removed it) gains nothing (CR 400.7); one that has rebound already
/// has a redundant second instance (CR 702.88c).
fn gains_rebound(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if let Some(spell) = item.subject.object() {
        g.add_effect(item.object, spell, EffectKind::AddKeywords(Keywords::of(&[Keyword::Rebound])), Duration::Indefinite);
    }
    Ok(())
}

/// "When Ojer Pakpatiq dies, return it to the battlefield tapped and transformed under its owner's control with three time
/// counters on it" (CR 712.14a, 122.6, 603.10a).
fn return_with_three_time_counters(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    effects::return_it_tapped_and_transformed_with_counters(g, item, &[(CounterKind::Time, 3)])
}

/// "{T}: Add {U}. Remove a time counter from this land." (CR 605.1a, 122.1): a mana ability, so the counter is removed as the
/// mana is added, with no stack.
fn add_blue_and_remove_a_time_counter(g: &mut dyn Game, asker: &Asker, _: Option<Color>) -> Result<(), Suspend> {
    g.add_mana(asker.controller, Mana::U, 1);
    g.remove_counters(asker.source, CounterKind::Time, 1);
    Ok(())
}

/// "Activate only if it has no time counters on it" (CR 602.5b): checked as the ability is activated, before its cost is paid.
fn no_time_counters(q: &dyn Query, asker: &Asker) -> bool {
    q.obj(asker.source).counters(CounterKind::Time) == 0
}

pub static BACK: CardDef = CardDef {
    abilities: &[
        mana_ability(TAP, add(Mana::U).by(add_blue_and_remove_a_time_counter), "{T}: Add {U}. Remove a time counter from this land."),
        ability(cost("{2}{U}, {T}"), effects::transform_self, "{2}{U}, {T}: Transform this land. Activate only if it has no time counters on it and only as a sorcery.")
            .sorcery_speed()
            .condition(no_time_counters),
    ],
    ..oracle::TEMPLE_OF_CYCLICAL_TIME
};

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            when_asked(EventMask::SPELL_CAST, you_cast_an_instant_from_your_hand),
            gains_rebound,
            "Whenever you cast an instant spell from your hand, it gains rebound. (Exile it as it resolves. At the beginning of your next upkeep, you may cast it from exile without paying its mana cost.)",
        ),
        triggered(
            cantrip_mechanics::triggers::THIS_DIES,
            return_with_three_time_counters,
            "When Ojer Pakpatiq dies, return it to the battlefield tapped and transformed under its owner's control with three time counters on it.",
        ),
    ],
    back: Some(&BACK),
    ..oracle::OJER_PAKPATIQ_DEEPEST_EPOCH
};
