use cantrip_api::*;
use cantrip_mechanics::{damage_target, effects, select, targets, triggers::THIS_ENTERS};
use cantrip_oracle as oracle;

/// "Another artifact" (CR 701.21a): an artifact permanent other than General Traag, the object whose text asks.
fn another_artifact(q: &dyn Query, choice: &Choice, _: u32) -> Verdict {
    Verdict::each(choice, |id| id != choice.source && q.chars(id).is(CardType::Artifact))
}

const ANOTHER_ARTIFACT: Selection = Selection::of(select::up_to(1), &[select::part(&(another_artifact as SelectFn), 0, "")]).called("other artifact");

/// "When General Traag enters, you may sacrifice another artifact. When you do, ..." (CR 701.21a, 603.12): the sacrifice is
/// chosen as the ability resolves, and the reflexive ability, with its target, exists only if an artifact was sacrificed.
fn sacrifice_for_damage(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if !effects::may_sacrifice(g, item.controller, item.object, ANOTHER_ARTIFACT)?.is_empty() {
        let this = g.card_id(&CARD);
        g.reflexive_trigger(item.object, item.controller, AbilityRef::delayed(this, 0), Target::None);
    }
    Ok(())
}

/// "General Traag deals 4 damage to target creature" (CR 120.2b, 608.2b/h): the original source, including its last-known
/// information, deals damage only to the still-legal target.
fn deal_four(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    damage_target(g, item, 0, 4);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(THIS_ENTERS, sacrifice_for_damage, "When General Traag enters, you may sacrifice another artifact. When you do, General Traag deals 4 damage to target creature.")],
    delayed: &[delayed_trigger(deal_four, "When you do, General Traag deals 4 damage to target creature.").targets(&[targets::CREATURE])],
    ..oracle::GENERAL_TRAAG_HEART_OF_STONE
};
