use cantrip_api::*;
use cantrip_oracle as oracle;

/// Whether `spell` is the first creature spell `player` casts this turn: a creature spell (as the face being cast), and no other
/// creature spell cast by them this turn (the ruling: Radagast need not have been on the battlefield for them, a creature spell
/// cast this turn before is the first, and a spell that was not a creature spell is not). Asked of this turn's casts, which hold a
/// spell once it has been cast, itself included.
fn first_creature_spell_you_cast_this_turn(q: &dyn Query, spell: ObjectId, player: PlayerId) -> bool {
    q.chars(spell).is(CardType::Creature) && !q.history().casts_by(player).any(|c| c.spell != spell && c.types.has(CardType::Creature))
}

/// "The first creature spell you cast each turn costs {2} less to cast" (CR 601.2f, 118.7): up to two generic mana of its total
/// cost, after any increases (the rulings); never a colored symbol, and not the mana cost or mana value of the spell. A spell that
/// is cast with an X has its X chosen first.
fn first_creature_spell_costs_two_less(q: &dyn Query, source: ObjectId, player: PlayerId, costed: Costed, _: &[Target]) -> CostDelta {
    match costed {
        Costed::Spell(spell) if player == q.controller(source) && first_creature_spell_you_cast_this_turn(q, spell, player) => CostDelta::less(2),
        _ => CostDelta::NONE,
    }
}

/// "... and can be cast as though it had flash" (CR 307.5, 702.8): the same spell, its controller's, from whatever zone they cast
/// it from.
fn first_creature_spell_you_cast(q: &dyn Query, asker: &Asker, spell: ObjectId, caster: PlayerId) -> bool {
    caster == asker.controller && first_creature_spell_you_cast_this_turn(q, spell, caster)
}

pub static CARD: CardDef = CardDef {
    cost_changes: &[first_creature_spell_costs_two_less],
    flash_rules: &[first_creature_spell_you_cast],
    ..oracle::RADAGAST_OF_RHOSGOBEL
};
