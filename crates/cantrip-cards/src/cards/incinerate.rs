use cantrip_api::*;
use cantrip_mechanics::{damage_target, history, targets};
use cantrip_oracle as oracle;

/// Deal damage, then prevent regeneration of creatures actually dealt damage (CR 120.3, 611.2a).
fn effect(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let before = g.history().head() as usize;
    damage_target(g, item, 0, 3);
    let dealt = history::permanents_dealt_damage_by(g.history(), item.object, before);
    g.end_sentence();
    let creatures: Vec<ObjectId> = dealt.into_iter().filter(|&id| g.is_on_battlefield(id) && g.chars(id).is(CardType::Creature)).collect();
    instruction(g, |g| {
        for creature in creatures {
            g.add_effect(item.object, creature, EffectKind::CantBeRegenerated, Duration::EndOfTurn);
        }
    });
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[targets::ANY], effect),
    ..oracle::INCINERATE
};
