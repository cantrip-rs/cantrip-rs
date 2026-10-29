use cantrip_api::*;
use cantrip_mechanics::{targets, triggers::THIS_ENTERS};
use cantrip_oracle as oracle;

/// "Each creature target opponent controls loses all abilities, becomes a Coward in addition to its other types, and has base
/// power and toughness 1/1" (CR 613.1d, 613.1f, 613.4b, 611.2c): only the creatures that opponent controls as this resolves
/// are affected, for as long as they last, however Colossus fares (the rulings); a creature they control later is not.
/// Nothing if the opponent is no longer a legal target (CR 608.2b). One instruction.
fn muddle_creatures(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(opponent) = targets::player(g, item, 0) else { return Ok(()) };
    let creatures: Vec<ObjectId> = g.permanents_of(opponent).into_iter().filter(|&id| g.chars(id).is(CardType::Creature)).collect();
    instruction(g, |g| {
        for creature in creatures {
            g.add_effect(item.object, creature, EffectKind::LoseAllAbilities, Duration::Indefinite);
            g.add_effect(item.object, creature, EffectKind::AddSubtype(Subtype::Coward), Duration::Indefinite);
            g.add_effect(item.object, creature, EffectKind::SetPt(1, 1), Duration::Indefinite);
        }
    });
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        THIS_ENTERS,
        muddle_creatures,
        "When this creature enters, each creature target opponent controls loses all abilities, becomes a Coward in addition to its other types, and has base power and toughness 1/1.",
    )
    .targets(&[targets::OPPONENT])],
    ..oracle::CURIOUS_COLOSSUS
};
