use cantrip_api::*;
use cantrip_mechanics::effects::{becomes_subtype_until_end_of_turn, plus_one_counter_on_this};
use cantrip_mechanics::targets::filters;
use cantrip_oracle as oracle;

/// "Put a +1/+1 counter on this Vehicle. It becomes a Spirit artifact creature in addition to its other types until end of turn"
/// (CR 122.1a, 205.1b, 301.7b, 611.2a, 613.1d): the counter, then the types, as two steps; it is an artifact already and stays one.
fn grow_and_become_a_spirit_creature(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    plus_one_counter_on_this(g, item)?;
    let this = item.object;
    instruction(g, |g| {
        g.add_effect(this, this, EffectKind::AddTypes(TypeSet::of(&[CardType::Artifact, CardType::Creature])), Duration::EndOfTurn);
        becomes_subtype_until_end_of_turn(g, this, this, Subtype::Spirit);
    });
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[ability(
        sacrifice(1, filters::another_creature_or_artifact),
        grow_and_become_a_spirit_creature,
        "Sacrifice another artifact or creature: Put a +1/+1 counter on this Vehicle. It becomes a Spirit artifact creature in addition to its other types until end of turn.",
    )],
    ..oracle::PHANTOM_TRAIN
};
