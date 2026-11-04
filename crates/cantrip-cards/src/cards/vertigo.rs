use cantrip_api::*;
use cantrip_mechanics::{damage_each_target, targets};
use cantrip_oracle as oracle;

/// "Vertigo deals 2 damage to target creature with flying. That creature loses flying until end of turn." (CR 608.2b, 611.2a).
fn damage_and_remove_flying(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(creature) = targets::object(g, item, 0) else { return Ok(()) };
    damage_each_target(g, item, 2);
    g.end_sentence();
    g.add_effect(item.object, creature, EffectKind::LoseKeywords(Keywords::of(&[Keyword::Flying])), Duration::EndOfTurn);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[targets::CREATURE_WITH_FLYING], damage_and_remove_flying),
    ..oracle::VERTIGO
};
