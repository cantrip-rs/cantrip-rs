use cantrip_api::*;
use cantrip_mechanics::{effects, spent, targets, triggers};
use cantrip_oracle as oracle;

/// "Whenever you cast a permanent spell using mana produced by The Myriad Pools" (CR 601.2h, 603.2, 106.3): a spell its controller
/// cast, a permanent spell as it is on the stack (CR 110.4a), some of whose total cost was paid with mana this permanent produced
/// - however little, and with whatever else. The ability is about the spell.
fn permanent_spell_cast_with_its_mana(q: &dyn Query, me: &Asker, event: &Event) -> Option<Fired> {
    match *event {
        Event::SpellCast { spell, player, .. } if player == me.controller && targets::is_permanent_card(&q.chars(spell)) && q.history().cast_using_mana_from(spell, me.source) => {
            Fired::about(Target::Object(spell))
        }
        _ => None,
    }
}

/// "Up to one other target permanent you control becomes a copy of that spell until end of turn" (CR 707.2, 611.2a): the target,
/// if it is still a legal target as the ability resolves (CR 608.2b), takes the copiable values of the spell that caused the
/// ability to trigger (`item.subject`), as it is then on the stack or, if it has left, as it last was (CR 608.2h). It is the
/// same object: it stays on the battlefield without entering or leaving. With no target chosen, nothing happens.
fn becomes_a_copy_of_the_spell(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let (Some(permanent), Some(spell)) = (targets::object(g, item, 0), item.subject.object()) else { return Ok(()) };
    let values = g.copiable_values(spell);
    g.become_copy(item.object, permanent, values, Duration::EndOfTurn);
    Ok(())
}

pub static BACK: CardDef = CardDef {
    abilities: &[
        mana_ability(TAP, add(Mana::U), "{T}: Add {U}."),
        triggered(
            when_asked(EventMask::SPELL_CAST, permanent_spell_cast_with_its_mana),
            becomes_a_copy_of_the_spell,
            "Whenever you cast a permanent spell using mana produced by The Myriad Pools, up to one other target permanent you control becomes a copy of that spell until end of turn.",
        )
        .targets(&[targets::UP_TO_ONE_ANOTHER_PERMANENT_YOU_CONTROL]),
    ],
    mana_told_apart: &[spent::this_permanents_mana_is_told_apart],
    ..oracle::THE_MYRIAD_POOLS
};

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(triggers::THIS_ENTERS, effects::you_mill_then_draw::<2, 2>, "When The Everflowing Well enters, mill two cards, then draw two cards."),
        triggered(triggers::YOUR_UPKEEP, effects::transform_self, "Descend 8 — At the beginning of your upkeep, if there are eight or more permanent cards in your graveyard, transform The Everflowing Well.")
            .condition(targets::you_have_permanent_cards_in_graveyard::<8>),
    ],
    back: Some(&BACK),
    ..oracle::THE_EVERFLOWING_WELL
};
