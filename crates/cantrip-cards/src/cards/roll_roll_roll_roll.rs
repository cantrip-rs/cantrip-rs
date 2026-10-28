use cantrip_api::*;
use cantrip_mechanics::{delayed, targets};
use cantrip_oracle as oracle;

/// "Up to one target creature or land you control" (CR 115.1): on the battlefield under this ability's controller's control.
fn creature_or_land_you_control(q: &dyn Query, asker: &Asker, t: Target) -> bool {
    matches!(t, Target::Object(id) if targets::permanent(q, id)
        && q.controller(id) == asker.controller
        && (q.chars(id).is(CardType::Creature) || q.chars(id).is(CardType::Land)))
}

/// "Exile up to one target creature or land you control. If you do, return it to the battlefield under its owner's control at
/// the beginning of the next end step." (CR 701.13, 603.7, 400.7): the delayed ability exists only if a permanent was exiled
/// (CR 608.2c) and is about it (CR 603.7c): a token has ceased to exist and does not return, and a card that left exile in
/// between is a new object and does not either.
fn blink(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(permanent) = targets::object(g, item, 0) else { return Ok(()) };
    let exiled = g.exile_all(&[permanent]);
    if !exiled.is_empty() {
        delayed::at_next_end_step_about(g, item, 0, &exiled);
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        chapter(1, blink, "I, II, III, IV \u{2014} Exile up to one target creature or land you control. If you do, return it to the battlefield under its owner's control at the beginning of the next end step.")
            .targets(&[target(creature_or_land_you_control).optional()]),
        chapter(2, blink, "I, II, III, IV \u{2014} Exile up to one target creature or land you control. If you do, return it to the battlefield under its owner's control at the beginning of the next end step.")
            .targets(&[target(creature_or_land_you_control).optional()]),
        chapter(3, blink, "I, II, III, IV \u{2014} Exile up to one target creature or land you control. If you do, return it to the battlefield under its owner's control at the beginning of the next end step.")
            .targets(&[target(creature_or_land_you_control).optional()]),
        chapter(4, blink, "I, II, III, IV \u{2014} Exile up to one target creature or land you control. If you do, return it to the battlefield under its owner's control at the beginning of the next end step.")
            .targets(&[target(creature_or_land_you_control).optional()]),
    ],
    delayed: &[delayed_trigger(delayed::return_exiled_to_battlefield, "Return it to the battlefield under its owner's control.")],
    ..oracle::ROLL_ROLL_ROLL_ROLL
};
