use cantrip_api::*;
use cantrip_mechanics::triggers::THIS_ENTERS;
use cantrip_mechanics::{effects, exile_and_return, exile_and_return_transformed, targets};
use cantrip_oracle as oracle;

/// "Exile Jill, then return it to the battlefield transformed under its owner's control" (CR 712.14a): the owner's, whoever
/// controlled Jill; nothing if she has left the battlefield already, since the new object is not the one the ability means (CR 400.7).
fn transform(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let owner = g.obj(item.object).owner;
    exile_and_return_transformed(g, item.object, owner)
}

/// "Tap all lands your opponents control. Exile Shiva, then return it to the battlefield (front face up)" (CR 701.26a, 712.14a):
/// the lands are tapped as one event, then Shiva is exiled and returns as a new object, Jill, under her owner's control (CR 400.7)
/// whose enters trigger happens again. Shiva is exiled only if she is still the permanent that chapter III triggered for.
fn cold_snap(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let opponents: Vec<PlayerId> = g.state().opponents(item.controller).collect();
    let lands: Vec<ObjectId> = opponents.into_iter().flat_map(|p| g.permanents_of(p)).filter(|&id| g.chars(id).is(CardType::Land)).collect();
    g.tap_together(&lands);
    if g.is_on_battlefield(item.object) {
        exile_and_return(g, &[item.object])?;
    }
    Ok(())
}

pub static BACK: CardDef = CardDef {
    abilities: &[
        chapter(1, effects::target_creature_cant_be_blocked_this_turn, "I \u{2014} Mesmerize \u{2014} Target creature can't be blocked this turn.").targets(&[targets::CREATURE]),
        chapter(2, effects::target_creature_cant_be_blocked_this_turn, "II \u{2014} Mesmerize \u{2014} Target creature can't be blocked this turn.").targets(&[targets::CREATURE]),
        chapter(3, cold_snap, "III \u{2014} Cold Snap \u{2014} Tap all lands your opponents control. Exile Shiva, then return it to the battlefield (front face up)."),
    ],
    ..oracle::SHIVA_WARDEN_OF_ICE
};

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(THIS_ENTERS, effects::return_first_target_to_hand, "When Jill enters, return up to one other target nonland permanent to its owner's hand.")
            .targets(&[up_to_one(targets::other_nonland_permanent_filter)]),
        ability(cost("{3}{U}{U}, {T}"), transform, "{3}{U}{U}, {T}: Exile Jill, then return it to the battlefield transformed under its owner's control. Activate only as a sorcery.")
            .sorcery_speed(),
    ],
    back: Some(&BACK),
    ..oracle::JILL_SHIVAS_DOMINANT
};
