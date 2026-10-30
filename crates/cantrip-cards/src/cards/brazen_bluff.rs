use cantrip_api::*;
use cantrip_mechanics::{choose, effects, targets};
use cantrip_oracle as oracle;

/// "Target noncreature spell an opponent controls." The target must still be a noncreature spell on the stack, controlled
/// by an opponent, as targets are chosen and when this spell resolves (CR 115.1, 608.2b).
fn noncreature_spell_an_opponent_controls(q: &dyn Query, asker: &Asker, t: Target) -> bool {
    (targets::NONCREATURE_SPELL.filter)(q, asker, t)
        && matches!(t, Target::Object(id) if q.controller(id) != asker.controller)
}

/// "That player may exile it. If they don't, you may cast a noncreature spell from your hand without paying its mana cost."
/// The target's controller chooses first; if the spell remains on the stack, this spell's controller gets the free-cast
/// choice (CR 608.2d, 608.2g, 118.9). The free cast uses an alternate cost, X is zero, and additional costs remain owed.
fn bluff(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(spell) = targets::object(g, item, 0) else { return Ok(()) };
    let player = g.controller(spell);
    if effects::ask_may(g, player, spell, Purpose::Exile)? {
        g.remove_spell(spell, CounterTo::Exile);
    } else {
        choose::may_cast_from_hand_free(g, item.controller, item.object, "noncreature spell", |g, way| {
            !g.card(way.face).types.has(CardType::Creature)
        })?;
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[target(noncreature_spell_an_opponent_controls)], bluff),
    ..oracle::BRAZEN_BLUFF
};
