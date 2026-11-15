use cantrip_api::*;
use cantrip_mechanics::{delayed, effects, pump, replacements, targets, triggers};
use cantrip_oracle as oracle;

/// The history serial after which `creature`'s damage counts for `source`'s Runesword effects about it this turn: the
/// resolution of the first of `source`'s Runesword abilities this turn that targeted it. Each resolution of the ability
/// made one rule about its creature (`effect`) and a target no longer legal makes none and no resolution (CR 608.2b), so
/// this turn's rules from `source` and this turn's resolutions of its ability (`Event::Resolved`) are in the same order,
/// one for one. Damage counts only after the effect began: "if the creature deals
/// damage ... this turn" is said of what happens from then on (CR 611.2c), as "when that creature leaves the battlefield
/// this turn" is. Two resolutions about the same creature are two effects whose sets are the earlier one's and a part of
/// it, so the earlier one's beginning is all either needs. None if `source` made no such effect this turn.
fn began(q: &dyn Query, source: ObjectId, creature: ObjectId) -> Option<u32> {
    let card = q.card_id(&CARD);
    let abouts = q.state().effects.iter().filter_map(|e| match e.kind {
        EffectKind::LastingRule { card: c, about, .. } if c == card && e.source == source => Some(about),
        _ => None,
    });
    let resolutions = q.history().this_turn().iter().filter_map(|r| match r.event {
        Event::Resolved { source: s, ability, .. } if s == source && ability == AbilityRef { card, index: 0 } => Some(r.serial),
        _ => None,
    });
    abouts.zip(resolutions).find(|&(about, _)| about == Target::Object(creature)).map(|(_, serial)| serial)
}

/// Whether `creature` dealt damage to the permanent `id` after the history serial `after` this turn (CR 120.3): damage
/// actually dealt, so damage prevented in full names nothing, and a redirection names the permanent it was dealt to.
/// `to_a_creature` asks also that `id` was a creature as it was dealt the damage, whatever it is now (CR 120.3d-e).
fn dealt_damage(q: &dyn Query, creature: ObjectId, after: u32, id: ObjectId, to_a_creature: bool) -> bool {
    q.history().this_turn().iter().any(|r| {
        matches!(r.event, Event::Damage { source, target: Target::Object(t), amount, kind, .. }
            if r.serial > after && source == creature && t == id && amount > 0 && (!to_a_creature || kind.dealt_to_creature()))
    })
}

/// "If the creature deals damage to a creature this turn, the creature dealt damage can't be regenerated this turn." (CR
/// 701.19c, 611.2c): a rule of the game about an open set - each permanent the targeted creature (`about`) deals damage to
/// while it was a creature, from the time the effect began - so a permanent dealt damage later is taken in, and it stays
/// in after it, the targeted creature or the source has left or lost its abilities (CR 613.1f).
fn cant_be_regenerated(q: &dyn Query, asker: &Asker, about: Target, id: ObjectId) -> bool {
    let Target::Object(creature) = about else { return false };
    began(q, asker.source, creature).is_some_and(|after| dealt_damage(q, creature, after, id, true))
}

/// "If a creature dealt damage by the targeted creature would die this turn, exile that creature instead." (CR 614.1a,
/// 700.4): a creature as it would die - whatever it was as it was dealt the damage - that the targeted creature (`affected`)
/// dealt damage to since the effect began. It outlives the targeted creature and the source (CR 611.2a).
fn exile_applies(q: &dyn Query, asker: &Asker, creature: ObjectId, proposal: &Proposal) -> bool {
    let Proposal::ZoneChange(z) = proposal else { return false };
    z.from.is(ZoneKind::Battlefield)
        && z.to.is(ZoneKind::Graveyard)
        && q.chars(z.object).is(CardType::Creature)
        && began(q, asker.source, creature).is_some_and(|after| dealt_damage(q, creature, after, z.object, false))
}

/// "{3}, {T}: Target attacking creature gets +2/+0 until end of turn. When that creature leaves the battlefield this turn,
/// sacrifice this artifact. If the creature deals damage to a creature this turn, the creature dealt damage can't be
/// regenerated this turn. If a creature dealt damage by the targeted creature would die this turn, exile that creature
/// instead." Four sentences (CR 608.2c): the pump (CR 611.2a), a delayed triggered ability watching that creature (CR
/// 603.7b-d), and two effects about the creatures it deals damage to from now on (CR 611.2c), which outlive it and this
/// artifact. A target no longer legal as this resolves does nothing (CR 608.2b).
///
/// The replacement is made once for each creature this artifact's abilities target in a turn: a second one about the same
/// creature would apply to the same event with the same result, never on its own (the first one's set holds the second's),
/// so the choice between them (CR 616.1) would not be one.
fn effect(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(creature) = targets::object(g, item, 0) else { return Ok(()) };
    pump(g, item.object, creature, 2, 0);
    g.end_sentence();
    delayed::this_turn_watching(g, item, creature);
    g.end_sentence();
    g.add_lasting_rule(item, 0, Target::Object(creature), Duration::EndOfTurn);
    g.end_sentence();
    let replacement = EffectKind::ReplacementRule { card: item.ability.card, index: 0 };
    let made = g.state().effects.iter().any(|e| e.kind == replacement && e.source == item.object && e.affected == creature);
    if !made {
        instruction(g, |g| g.add_effect(item.object, creature, replacement, Duration::EndOfTurn));
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[ability(
        cost("{3}").and(TAP),
        effect,
        "{3}, {T}: Target attacking creature gets +2/+0 until end of turn. When that creature leaves the battlefield this turn, sacrifice this artifact. If the creature deals damage to a creature this turn, the creature dealt damage can't be regenerated this turn. If a creature dealt damage by the targeted creature would die this turn, exile that creature instead.",
    )
    .targets(&[targets::ATTACKING_CREATURE])],
    delayed: &[triggered(
        triggers::THIS_LEAVES,
        effects::sacrifice_self,
        "When that creature leaves the battlefield this turn, sacrifice this artifact.",
    )],
    lasting_rules: &[LastingRule {
        text: "If the creature deals damage to a creature this turn, the creature dealt damage can't be regenerated this turn.",
        rule: LastingRuleKind::CantBeRegenerated,
        applies: cant_be_regenerated,
    }],
    granted_replacements: &[granted_replacement(
        ReplacementKind::ZoneChange,
        exile_applies,
        replacements::exile_instead_apply_granted,
        "exiles a creature the targeted creature dealt damage to instead of letting it die",
    )
    .text("If a creature dealt damage by the targeted creature would die this turn, exile that creature instead.")],
    ..oracle::RUNESWORD
};
