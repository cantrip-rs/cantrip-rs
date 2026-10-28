use cantrip_api::*;
use cantrip_mechanics::effects;
use cantrip_mechanics::triggers::{THIS_ATTACKS, THIS_ENTERS};
use cantrip_mechanics::{targets, tokens};
use cantrip_oracle as oracle;

/// "When Dáin enters, create a colorless Equipment artifact token named Axe ... When you do, attach it to target creature you
/// control." The reflexive ability (CR 603.12) triggers only if the token was created, and is about that token.
fn create_axe(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let made = tokens::create(g, item.controller, &tokens::AXE, 1)?;
    let Some(&axe) = made.first() else { return Ok(()) };
    let this = g.card_id(&CARD);
    g.reflexive_trigger(item.object, item.controller, AbilityRef::delayed(this, 0), Target::Object(axe));
    Ok(())
}

/// "When you do, attach it to target creature you control": the Axe is attached if it is still on the battlefield as this
/// resolves (CR 701.3b), to the creature if it is still a legal target (CR 608.2b).
fn attach_it(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Target::Object(axe) = item.subject else { return Ok(()) };
    if let Some(creature) = targets::object(g, item, 0) {
        g.attach(axe, creature);
    }
    Ok(())
}

/// "Whenever Dáin attacks, each equipped attacking creature gains double strike until end of turn": the attacking creatures
/// with an Equipment attached as this resolves (CR 301.5), Dáin among them if it is equipped (CR 611.2c).
fn double_strike_for_equipped(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let attackers: Vec<ObjectId> = g.state().attackers.iter().map(|a| a.id).collect();
    let equipment: Vec<ObjectId> =
        g.state().battlefield().iter().copied().filter(|&id| g.chars(id).has_subtype(Subtype::Equipment)).collect();
    let equipped: Vec<ObjectId> = attackers.into_iter().filter(|&creature| equipment.iter().any(|&e| g.attached_to(e) == Some(creature))).collect();
    instruction(g, |g| {
        for creature in equipped {
            effects::gain_until_end_of_turn(g, item.object, creature, Keyword::DoubleStrike);
        }
    });
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            THIS_ENTERS,
            create_axe,
            "When Dáin enters, create a colorless Equipment artifact token named Axe with \"Equipped creature gets +1/+0\" and equip {2}. When you do, attach it to target creature you control.",
        ),
        triggered(THIS_ATTACKS, double_strike_for_equipped, "Whenever Dáin attacks, each equipped attacking creature gains double strike until end of turn."),
    ],
    delayed: &[delayed_trigger(attach_it, "When you do, attach it to target creature you control.").targets(&[targets::CREATURE_YOU_CONTROL])],
    ..oracle::D_IN_IRONFOOT
};

pub static EXTRA: &[&CardDef] = &[&tokens::AXE];
