use cantrip_api::*;
use cantrip_mechanics::effects::ask_may;
use cantrip_mechanics::{targets, triggers};
use cantrip_oracle as oracle;

const ASK: Question = Question::new("Damage", "have Requiem Monolith deal 1 damage to this creature?", "It draws you cards equal to the damage", "Deal 1 damage", "Don't");

/// "Whenever this creature is dealt damage, you draw that many cards and lose that much life" (CR 120.2, 603.2c): "that many" is
/// the exact matched damage dealt at once; "you" is the creature's controller.
fn draw_that_many_and_lose_that_much(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let amount = item.matched_damage_amount();
    g.draw(item.controller, u32::try_from(amount).unwrap_or(u32::MAX));
    g.lose_life(item.controller, amount as i32);
    Ok(())
}

/// Until end of turn the target creature gains the damage trigger (CR 611.2a, 613.1f), then its controller may have this
/// artifact deal 1 damage to it (CR 120.3): the choice is that player's, not the Monolith's controller's. A target that is
/// no longer legal does nothing (CR 608.2b).
fn grant_and_ping(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(creature) = targets::object(g, item, 0) else { return Ok(()) };
    let this = g.card_id(&CARD);
    g.one_instruction(&mut |g| {
        g.add_effect(item.object, creature, EffectKind::AddAbility(AbilityRef::delayed(this, 0)), Duration::EndOfTurn);
        Ok(())
    })?;
    let owner = g.controller(creature);
    if ask_may(g, owner, item.object, Purpose::Ask(&ASK))? {
        g.damage(item.object, Target::Object(creature), 1);
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[ability(TAP, grant_and_ping, "{T}: Until end of turn, target creature gains \"Whenever this creature is dealt damage, you draw that many cards and lose that much life.\" That creature's controller may have this artifact deal 1 damage to it. Activate only as a sorcery.")
        .sorcery_speed()
        .targets(&[targets::CREATURE])],
    delayed: &[triggered(triggers::THIS_DEALT_DAMAGE, draw_that_many_and_lose_that_much, "Whenever this creature is dealt damage, you draw that many cards and lose that much life.").sums_per_subject().damage_amount()],
    ..oracle::REQUIEM_MONOLITH
};
