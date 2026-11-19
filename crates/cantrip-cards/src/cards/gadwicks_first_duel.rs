use cantrip_api::*;
use cantrip_mechanics::{copy, delayed, effects, targets, tokens, triggers};
use cantrip_oracle as oracle;

/// "An instant or sorcery spell with mana value 3 or less" (CR 202.3, 603.2): the spell its controller casts, read as it stands on
/// the stack once cast, so an X is what was chosen for it (CR 202.3e). The spell rides along as the trigger's subject.
fn you_cast_an_instant_or_sorcery_with_mana_value_3_or_less(q: &dyn Query, me: &Asker, event: &Event) -> Option<Fired> {
    let fired = triggers::you_cast_instant_or_sorcery(q, me, event)?;
    let Event::SpellCast { spell, .. } = event else { return None };
    (q.mana_value(*spell) <= 3).then_some(fired)
}

/// The Role under the Saga's controller, attached to the target; nothing when no target was chosen or it is no longer legal (CR 608.2b, 115.1d).
fn create_cursed_role(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(creature) = targets::object(g, item, 0) else { return Ok(()) };
    tokens::create_role(g, &tokens::CURSED_ROLE, item.controller, creature)?;
    Ok(())
}

/// Chapter III's delayed ability (CR 603.7b, 707.10): it triggers once on the next such spell its controller casts this turn, and
/// ends with the turn if none is. The copy is not cast.
const COPY_THE_NEXT_INSTANT_OR_SORCERY_WITH_MANA_VALUE_3_OR_LESS: AbilityDef = triggered(
    when_asked(EventMask::SPELL_CAST, you_cast_an_instant_or_sorcery_with_mana_value_3_or_less),
    copy::copy_the_cast_spell,
    "When you next cast an instant or sorcery spell with mana value 3 or less this turn, copy that spell. You may choose new targets for the copy.",
);

pub static CARD: CardDef = CardDef {
    abilities: &[
        chapter(
            1,
            create_cursed_role,
            "I \u{2014} Create a Cursed Role token attached to up to one target creature.",
        )
        .targets(&[targets::CREATURE.optional()]),
        chapter(2, effects::scry_two, "II \u{2014} Scry 2."),
        chapter(
            3,
            delayed::watch_for_the_next,
            "III \u{2014} When you next cast an instant or sorcery spell with mana value 3 or less this turn, copy that spell. You may choose new targets for the copy.",
        ),
    ],
    delayed: &[COPY_THE_NEXT_INSTANT_OR_SORCERY_WITH_MANA_VALUE_3_OR_LESS],
    ..oracle::GADWICKS_FIRST_DUEL
};

pub static EXTRA: &[&CardDef] = &[&tokens::CURSED_ROLE];
