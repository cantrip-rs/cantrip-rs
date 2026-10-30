use cantrip_api::*;
use cantrip_mechanics::{delayed, haste_until_end_of_turn, targets, triggers};
use cantrip_oracle as oracle;

/// "If you've cast a noncreature spell this turn" (CR 601.2i, 603.4): this checks the current turn's cast history when the
/// trigger would trigger and again as it resolves. The history includes earlier spells from this turn before Gray and Adira
/// entered; copies and spells cast by an opponent do not.
fn cast_noncreature_this_turn(q: &dyn Query, asker: &Asker) -> bool {
    q.history()
        .casts_by(asker.controller)
        .any(|cast| !cast.is(CardType::Creature))
}

/// "Create a token that's a copy of target creature you control, except it isn't legendary. It gains haste until end of
/// turn. Sacrifice it at the beginning of the next end step" (CR 707.2, 707.9b, 702.10b, 611.2a, 603.7c, 400.7). The copy
/// exception removes legendary from its copiable values; haste is a separate end-of-turn effect. The delayed ability tracks
/// exactly the token objects created here, not a later object with the same name.
fn copy_hasty_creature(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(creature) = targets::object(g, item, 0) else { return Ok(()) };
    let values = g.copiable_values(creature).nonlegendary();
    let created = g.create_token_copies_of(values, item.controller, 1)?;
    g.end_sentence();
    instruction(g, |g| {
        for &token in &created {
            haste_until_end_of_turn(g, item.object, token);
        }
    });
    g.end_sentence();
    delayed::at_next_end_step_about(g, item, 0, &created);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        triggers::BEGINNING_OF_YOUR_COMBAT,
        copy_hasty_creature,
        "At the beginning of combat on your turn, if you've cast a noncreature spell this turn, create a token that's a copy of target creature you control, except it isn't legendary. It gains haste until end of turn. Sacrifice it at the beginning of the next end step.",
    )
    .condition(cast_noncreature_this_turn)
    .targets(&[targets::CREATURE_YOU_CONTROL])],
    delayed: &[delayed::SACRIFICE_CREATED],
    ..oracle::GRAY_AND_ADIRA_FOREVER_JOINED
};
