use cantrip_api::*;
use cantrip_mechanics::{discard_at_random, effects, targets};
use cantrip_oracle as oracle;

const ASK: Question = Question::new("ForgoCombatDamage", "have the enchanted creature assign no combat damage this turn so the defending player discards a card at random?", "Chosen as Cloak of Confusion's ability resolves", "Forgo damage", "Don't");

/// The unblocked event includes creatures put onto the battlefield attacking (CR 509.1h).
fn unblocked_enchanted(q: &dyn Query, asker: &Asker, event: &Event) -> Option<Fired> {
    let Event::Unblocked { attacker, .. } = event else { return None };
    if q.attached_to(asker.source) != Some(*attacker) { return None; }
    let defender = q.defending_player(*attacker)?;
    Some(Fired { subject: Target::Object(*attacker), data: [u32::from(defender.0), 0] })
}

/// The choice creates an effect lasting this turn; the defending player discards if it was chosen (CR 611.2a).
fn effect(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if effects::ask_may(g, item.controller, item.object, Purpose::Ask(&ASK))? {
        let Target::Object(creature) = item.subject else { return Ok(()) };
        let defender = g.state().attackers.iter().find(|a| a.id == creature)
            .map_or(PlayerId(item.data[0] as u8), |a| a.defending);
        g.add_effect(item.object, creature, EffectKind::NoCombatDamageAssignment, Duration::EndOfTurn);
        g.end_sentence();
        discard_at_random(g, defender, 1);
    }
    g.end_sentence();
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        when_asked(EventMask::UNBLOCKED, unblocked_enchanted),
        effect,
        "Whenever enchanted creature attacks and isn't blocked, you may have it assign no combat damage this turn. If you do, defending player discards a card at random.",
    )],
    enchant: Some(targets::CREATURE_YOU_CONTROL),
    ..oracle::CLOAK_OF_CONFUSION
};
