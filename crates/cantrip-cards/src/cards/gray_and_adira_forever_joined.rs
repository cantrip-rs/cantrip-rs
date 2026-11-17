use cantrip_api::*;
use cantrip_mechanics::{copy, targets, triggers};
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
/// turn. Sacrifice it at the beginning of the next end step" (`copy::create_token_copy_with_haste_sacrificed_at_end_step`,
/// CR 707.2, 707.9b, 611.2a, 603.7c): the copy exception removes legendary from its copiable values.
fn copy_hasty_creature(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(creature) = targets::object(g, item, 0) else { return Ok(()) };
    let values = g.copiable_values(creature).nonlegendary();
    copy::create_token_copy_with_haste_sacrificed_at_end_step(g, item, values, Duration::EndOfTurn, 0)
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        triggers::BEGINNING_OF_YOUR_COMBAT,
        copy_hasty_creature,
        "At the beginning of combat on your turn, if you've cast a noncreature spell this turn, create a token that's a copy of target creature you control, except it isn't legendary. It gains haste until end of turn. Sacrifice it at the beginning of the next end step.",
    )
    .condition(cast_noncreature_this_turn)
    .targets(&[targets::CREATURE_YOU_CONTROL])],
    delayed: &[copy::SACRIFICE_IT_AT_END_STEP],
    ..oracle::GRAY_AND_ADIRA_FOREVER_JOINED
};
