use cantrip_api::*;
use cantrip_mechanics::{damage_target, targets};
use cantrip_oracle as oracle;

/// "Lava Burst deals X damage to any target."
fn effect(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    damage_target(g, item, 0, item.x);
    Ok(())
}

/// "If Lava Burst would deal damage to a creature, that damage can't be prevented or dealt instead to another permanent or
/// player": damage Lava Burst itself would deal to a creature, judged at the recipient the damage would be dealt to now
/// (CR 616.1f) - damage to a player redirected to a creature is such damage from there on, and damage to a player is neither
/// (its rulings). The sentence is part of the spell's ability (CR 113.3a), about what it deals as it resolves; Lava Burst deals
/// damage only then, on the stack, where an instant's or sorcery's abilities function (CR 113.6), so a rule of the spell
/// there covers exactly the damage the sentence names: none of it is prevented (CR 615.12), and no redirection effect applies
/// to it (CR 614.9).
fn its_damage_to_a_creature(q: &dyn Query, asker: &Asker, source: ObjectId, target: Target, _: bool) -> bool {
    source == asker.source && matches!(target, Target::Object(id) if q.chars(id).is(CardType::Creature))
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[targets::ANY], effect),
    unpreventable_damage: &[its_damage_to_a_creature],
    unredirectable_damage: &[its_damage_to_a_creature],
    ..oracle::LAVA_BURST
};
