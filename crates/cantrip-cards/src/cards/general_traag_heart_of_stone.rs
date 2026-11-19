use cantrip_api::*;
use cantrip_mechanics::{effects, select, targets, triggers::THIS_ENTERS};
use cantrip_oracle as oracle;

/// "Another artifact" (CR 701.21a): an artifact permanent other than General Traag, the object whose text asks.
fn another_artifact(q: &dyn Query, choice: &Choice, _: u32) -> Verdict {
    Verdict::each(choice, |id| id != choice.source && q.chars(id).is(CardType::Artifact))
}

const ANOTHER_ARTIFACT: Selection = Selection::of(select::up_to(1), &[select::part(&(another_artifact as SelectFn), 0, "")]).called("other artifact");

/// "When General Traag enters, you may sacrifice another artifact. When you do, ..." (CR 701.21a, 603.12): the sacrifice is
/// chosen as the ability resolves, and the reflexive ability, with its target, exists only if an artifact was sacrificed.
fn sacrifice_for_damage(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    effects::may_sacrifice_when_you_do(g, item, ANOTHER_ARTIFACT)
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(THIS_ENTERS, sacrifice_for_damage, "When General Traag enters, you may sacrifice another artifact. When you do, General Traag deals 4 damage to target creature.")],
    delayed: &[delayed_trigger(effects::damage_first_target::<4>, "When you do, General Traag deals 4 damage to target creature.").targets(&[targets::CREATURE])],
    ..oracle::GENERAL_TRAAG_HEART_OF_STONE
};
