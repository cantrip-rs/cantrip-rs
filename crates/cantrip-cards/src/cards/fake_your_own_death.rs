use cantrip_api::*;
use cantrip_mechanics::triggers::THIS_DIES;
use cantrip_mechanics::{effects, pump, targets, tokens};
use cantrip_oracle as oracle;

/// "Until end of turn, target creature gets +2/+0 and gains 'When this creature dies, return it to the battlefield tapped under
/// its owner's control and you create a Treasure token'" (CR 611.2a, 613.1f): both are continuous effects of the resolving
/// spell on the target, one instruction; nothing if the target has left the battlefield (CR 608.2b, 400.7).
fn pump_and_grant_return(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(creature) = targets::object(g, item, 0) else { return Ok(()) };
    g.one_instruction(&mut |g| {
        pump(g, item.object, creature, 2, 0);
        let this = g.card_id(&CARD);
        g.add_effect(item.object, creature, EffectKind::AddAbility(AbilityRef::delayed(this, 0)), Duration::EndOfTurn);
        Ok(())
    })
}

/// "Return it to the battlefield tapped under its owner's control and you create a Treasure token" (CR 110.2a, 111.10a): "you"
/// is the creature's controller as it died, who gets the Treasure even when the creature was a token and returns nothing.
fn return_it_tapped_and_create_a_treasure(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    effects::return_it_tapped(g, item)?;
    tokens::create_treasure(g, item.controller)
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[targets::CREATURE], pump_and_grant_return),
    delayed: &[triggered(
        THIS_DIES,
        return_it_tapped_and_create_a_treasure,
        "When this creature dies, return it to the battlefield tapped under its owner's control and you create a Treasure token.",
    )],
    ..oracle::FAKE_YOUR_OWN_DEATH
};

pub static EXTRA: &[&CardDef] = &[&tokens::TREASURE];
