use cantrip_api::*;
use cantrip_mechanics::triggers;
use cantrip_oracle as oracle;

/// "Spells you cast have delve." (CR 702.66a, 601.2f-h): any spell `caster`, Teval's controller, casts, from any zone. Delve
/// applies after the total cost is set, so it pays the generic mana of that cost and no other. Only while Teval is on the
/// battlefield with its abilities (a static ability, CR 113.6); a spell an opponent casts gets nothing.
fn spells_delve(q: &dyn Query, source: ObjectId, caster: PlayerId, _spell: ObjectId) -> PayWith {
    if q.controller(source) == caster {
        PayWith { delve: true, ..PayWith::NONE }
    } else {
        PayWith::NONE
    }
}

/// "You lose life equal to its mana value." (CR 119.3, 202.3e): the spell's mana value as this ability resolves, or as it last was
/// on the stack if it has left (CR 608.2h: the ability still resolves if the spell is countered), an {X} in its cost counting as
/// the X announced. Life is lost, not paid, so a total past what the player has still takes it. The ability is about the
/// spell (`triggers::you_cast_a_spell_about_it`: cast, not copied, CR 707.10).
fn lose_life_equal_to_its_mana_value(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Target::Object(spell) = item.subject else { return Ok(()) };
    let value = g.mana_value(spell) as i32;
    g.lose_life(item.controller, value);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        when_asked(EventMask::SPELL_CAST, triggers::you_cast_a_spell_about_it),
        lose_life_equal_to_its_mana_value,
        "Whenever you cast a spell, you lose life equal to its mana value.",
    )],
    grants_pay_with: &[spells_delve],
    ..oracle::TEVAL_ARBITER_OF_VIRTUE
};
