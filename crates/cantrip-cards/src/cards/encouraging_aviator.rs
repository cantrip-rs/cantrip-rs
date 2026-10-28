use cantrip_api::*;
use cantrip_mechanics::{effects, prepare, targets, triggers};
use cantrip_oracle as oracle;

/// "Target creature gains flying until end of turn" (CR 611.2a, 702.9b): nothing if the target is no longer legal (CR 608.2b).
fn jump_effect(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(creature) = targets::object(g, item, 0) else { return Ok(()) };
    effects::gain_until_end_of_turn(g, item.object, creature, Keyword::Flying);
    Ok(())
}

/// Encouraging Aviator's prepare spell (CR 722): not a card here, only ever a copy.
pub static JUMP: CardDef = CardDef {
    name: "Jump",
    cost: mana("{U}"),
    types: TypeSet::of(&[CardType::Instant]),
    spell: spell(&[targets::CREATURE], jump_effect),
    text: "Target creature gains flying until end of turn.",
    ..CardDef::BLANK
};

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(triggers::THIS_ATTACKS, prepare::become_prepared, "Whenever this creature attacks, it becomes prepared.")],
    prepare: Some(&JUMP),
    ..oracle::ENCOURAGING_AVIATOR
};

pub static EXTRA: &[&CardDef] = &[&JUMP];
