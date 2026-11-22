use cantrip_api::*;
use cantrip_mechanics::library::object_has_name;
use cantrip_mechanics::{delayed, effects};
use cantrip_oracle as oracle;

/// I: "Surveil 3." (CR 701.25.)
fn chapter_one(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    effects::surveil(g, item, 3)
}

/// The delayed ability's effect: "Copy it" (CR 707.10), the spell that was cast riding along as the trigger's subject. The copy is
/// not cast, and "except it isn't legendary" is part of its copiable values (CR 707.9b, 707.2), so the permanent it becomes as it
/// resolves is a token (CR 707.10a) that is not legendary. The text lets
/// nobody choose new targets for it (CR 707.10c): a creature spell cast for its mutate cost keeps its target (CR 702.140a).
fn copy_the_spell_nonlegendary(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(spell) = item.subject.object() else { return Ok(()) };
    let values = g.copiable_values(spell).nonlegendary();
    g.copy_spell_as(spell, item.controller, false, values)?;
    Ok(())
}

/// III: "Choose a card name" (CR 201.4), any card name; then its "this turn" ability is set up (CR 603.7b), the card's delayed
/// ability 0, carrying the name this resolution chose (CR 607.2d, 603.7). The name is the delayed ability's, not the Saga's:
/// the ability exists apart from the Saga (CR 113.7a), so it keeps the name once the Saga has been sacrificed (CR 714.4) or has
/// left before this chapter resolved, and a copy of this chapter (Peter Parker's Camera) names a card of its own for a delayed
/// ability of its own (CR 707.10). With no name chosen, there is no creature with the chosen name, and nothing is set up.
fn chapter_three(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(name) = effects::name_a_card(g, item, NameKind::ANY, None)? else { return Ok(()) };
    g.delayed_trigger_carrying(DelayedWhen::ThisTurn, item.object, item.controller, AbilityRef::delayed(item.ability.card, 0), Target::None, ChosenValue::Name(name));
    Ok(())
}

/// "Whenever a creature with the chosen name deals combat damage to a player this turn": any creature, whoever controls it, that
/// has the name this ability carries, chosen by the chapter that created it (CR 607.2d, `Query::delayed_chosen`), as it deals the damage (CR 603.10a: judged as it was, so a creature that died in
/// the same combat damage step still triggers), damage of any amount above zero to any player.
fn creature_with_the_chosen_name_deals_combat_damage_to_a_player(q: &dyn Query, _: &Asker, event: &Event) -> Option<Fired> {
    let Event::Damage { source, target: Target::Player(_), amount, kind, .. } = *event else { return None };
    let name = q.delayed_chosen()?.card()?;
    if kind.is_combat() && amount > 0 && q.chars(source).is(CardType::Creature) && object_has_name(q, source, name) {
        Fired::PLAIN
    } else {
        None
    }
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        chapter(1, chapter_one, "I \u{2014} Surveil 3."),
        // II sets up the delayed ability "When you next cast a creature spell this turn, copy it, except the copy isn't legendary"
        // (CR 603.7b), the card's delayed ability 1. The Saga's own chapter is not a creature spell, and the ability ends with the
        // turn if none is cast.
        chapter(2, delayed::watch_for_the_next::<1>, "II \u{2014} When you next cast a creature spell this turn, copy it, except the copy isn't legendary."),
        chapter(3, chapter_three, "III \u{2014} Choose a card name. Whenever a creature with the chosen name deals combat damage to a player this turn, draw a card."),
    ],
    delayed: &[
        triggered(
            when_asked(EventMask::DAMAGE, creature_with_the_chosen_name_deals_combat_damage_to_a_player),
            |g, item| effects::draw(g, item, 1),
            "Whenever a creature with the chosen name deals combat damage to a player this turn, draw a card.",
        ),
        triggered(
            cantrip_mechanics::triggers::CREATURE_SPELL_YOU_CAST,
            copy_the_spell_nonlegendary,
            "When you next cast a creature spell this turn, copy it, except the copy isn't legendary.",
        ),
    ],
    ..oracle::THE_CLONE_SAGA
};
