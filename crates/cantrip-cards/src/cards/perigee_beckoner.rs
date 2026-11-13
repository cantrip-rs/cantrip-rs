use cantrip_api::*;
use cantrip_mechanics::triggers::{THIS_DIES, THIS_ENTERS};
use cantrip_mechanics::{effects, pump, targets, warp};
use cantrip_oracle as oracle;

/// The return trigger is the card's second delayed ability, after warp's exile (`warp::EXILE` is the first).
const RETURN_TRIGGER: u8 = 1;

/// "When this creature enters, until end of turn, another target creature you control gets +2/+0 and gains 'When this creature
/// dies, return it to the battlefield tapped under its owner's control'" (CR 611.2a, 613.1f): both are continuous effects of the
/// resolving ability on the target, one instruction; nothing if the target has left the battlefield (CR 608.2b, 400.7).
fn pump_and_grant_return(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(creature) = targets::object(g, item, 0) else { return Ok(()) };
    g.one_instruction(&mut |g| {
        pump(g, item.object, creature, 2, 0);
        let this = g.card_id(&CARD);
        g.add_effect(item.object, creature, EffectKind::AddAbility(AbilityRef::delayed(this, RETURN_TRIGGER)), Duration::EndOfTurn);
        Ok(())
    })
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        THIS_ENTERS,
        pump_and_grant_return,
        "When this creature enters, until end of turn, another target creature you control gets +2/+0 and gains \"When this creature dies, return it to the battlefield tapped under its owner's control.\"",
    )
    .targets(&[targets::OTHER_CREATURE_YOU_CONTROL])],
    ways: &[warp::way(mana("{1}{B}"), "Warp {1}{B}")],
    end_step_riders: warp::RIDERS,
    delayed: &[
        warp::EXILE,
        triggered(THIS_DIES, effects::return_it_tapped, "When this creature dies, return it to the battlefield tapped under its owner's control."),
    ],
    ..oracle::PERIGEE_BECKONER
};
