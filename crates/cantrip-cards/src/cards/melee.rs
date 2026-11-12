use cantrip_api::*;
use cantrip_mechanics::combat;
use cantrip_oracle as oracle;

/// "Cast this spell only during combat on your turn before blockers are declared." (CR 601.3): the beginning of combat or
/// declare attackers step of any combat phase (the 2013 ruling), of a turn of the caster's.
fn during_combat_on_your_turn_before_blockers(q: &dyn Query, asker: &Asker) -> bool {
    combat::cast_only_before_blockers(q, asker) && q.state().turn.active == asker.controller
}

/// "You choose which creatures block this combat and how those creatures block." (CR 509.1, 101.1): you make this combat's
/// declaration of blockers in the defending player's place, under the same restrictions and requirements (Odric, Master
/// Tactician's rulings: the declaration must still be legal, and you may have no creature block), until the combat phase
/// ends (`Duration::EndOfCombat`). "Whenever a creature attacks and isn't blocked this combat, ..." is a delayed triggered
/// ability made as it resolves (CR 603.7), lasting as long.
fn melee(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    g.add_player_effect(item.controller, item.object, EffectKind::ChoosesBlocks, Duration::EndOfCombat);
    g.end_sentence();
    g.delayed_trigger(DelayedWhen::ThisCombat, item.object, item.controller, AbilityRef::delayed(item.ability.card, 0), Target::None);
    Ok(())
}

/// "Whenever a creature attacks and isn't blocked this combat" (CR 509.3g, 603.7): an attacker no creature was declared to
/// block, about that attacker. The delayed ability is `DelayedWhen::ThisCombat`, so it ends with the combat phase Melee was
/// cast in and a later combat phase of the turn is not it.
fn attacks_and_isnt_blocked_this_combat(_q: &dyn Query, _asker: &Asker, event: &Event) -> Option<Fired> {
    let Event::Unblocked { attacker, .. } = event else { return None };
    Fired::about(Target::Object(*attacker))
}

/// "Untap it and remove it from combat." (CR 701.26b, 506.4): nothing for a creature that has left the battlefield.
fn untap_and_remove_from_combat(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(creature) = item.subject.object() else { return Ok(()) };
    g.untap(creature);
    g.remove_from_combat(creature);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[], melee),
    cast_only_if: Some(during_combat_on_your_turn_before_blockers),
    delayed: &[triggered(
        when_asked(EventMask::UNBLOCKED, attacks_and_isnt_blocked_this_combat),
        untap_and_remove_from_combat,
        "Whenever a creature attacks and isn't blocked this combat, untap it and remove it from combat.",
    )],
    ..oracle::MELEE
};
