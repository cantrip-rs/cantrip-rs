use cantrip_api::*;
use cantrip_mechanics::{effects::gain_until_end_of_turn, targets};
use cantrip_oracle as oracle;

/// "Creatures with flying can't attack you" (CR 508.1c): a creature with flying that an opponent of Storm's controller
/// controls is kept from attacking, judged as attackers are declared; in a duel that opponent attacks only Storm's
/// controller. A creature already attacking stays so if it gains flying afterwards (CR 506.4a).
fn flyers_cant_attack_you(q: &dyn Query, source: ObjectId, creature: ObjectId) -> bool {
    q.controller(creature) == q.controller(source) || !q.chars(creature).has(Keyword::Flying)
}

/// "Creatures with flying can't ... block creatures you control" (CR 509.1b): of a possible block, a blocker with flying
/// may not block an attacker its own controller does not control and Storm's controller does.
fn flyers_cant_block_yours(q: &dyn Query, source: ObjectId, blocker: ObjectId, attacker: ObjectId) -> bool {
    if q.controller(attacker) != q.controller(source) {
        return true;
    }
    !q.chars(blocker).has(Keyword::Flying)
}

/// The creatures on the battlefield that `spell` targeted as it was cast (CR 601.2c, 115.10): its `BecameTarget` records,
/// which stay when the spell has since left the stack.
fn creatures_targeted_by(q: &dyn Query, spell: ObjectId) -> Vec<ObjectId> {
    q.history()
        .select(EventQuery::kind(EventMask::BECAME_TARGET))
        .filter_map(|r| match r.event {
            Event::BecameTarget { target: Target::Object(id), by, .. } if by == spell => Some(id),
            _ => None,
        })
        .filter(|&id| targets::permanent(q, id))
        .filter(|&id| q.chars(id).is(CardType::Creature))
        .collect()
}

/// "Whenever you cast a spell that targets one or more creatures": the cast is the event, the spell the ability is about
/// (`StackItem::subject`). Its targets are read from the stack: the cast is checked before they are recorded as targets.
fn you_cast_a_spell_that_targets_creatures(q: &dyn Query, me: &Asker, event: &Event) -> Option<Fired> {
    let Event::SpellCast { spell, player, .. } = event else { return None };
    let item = q.state().stack.iter().filter(|i| i.kind == StackKind::Spell).find(|i| i.object == *spell)?;
    let mut targeted = q.state().targets(item).iter().filter_map(|t| t.object()).filter(|&id| targets::permanent(q, id));
    if *player != me.controller || !targeted.any(|id| q.chars(id).is(CardType::Creature)) {
        return None;
    }
    Fired::about(Target::Object(*spell))
}

/// "Those creatures gain flying until end of turn" (CR 611.2a): the creatures the spell targeted that are still on the
/// battlefield as the ability resolves, whether or not the spell is still on the stack.
fn those_creatures_gain_flying(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let creatures = item.subject.object().map(|spell| creatures_targeted_by(g, spell)).unwrap_or_default();
    // One instruction for all of them (CR 611.2c).
    instruction(g, |g| {
        for id in creatures {
            gain_until_end_of_turn(g, item.object, id, Keyword::Flying);
        }
    });
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        when_asked(EventMask::SPELL_CAST, you_cast_a_spell_that_targets_creatures),
        those_creatures_gain_flying,
        "Whenever you cast a spell that targets one or more creatures, those creatures gain flying until end of turn.",
    )],
    attack_rules: &[flyers_cant_attack_you],
    block_rules: &[flyers_cant_block_yours],
    ..oracle::STORM_WINDRIDER
};
