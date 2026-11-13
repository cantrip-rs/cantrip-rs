use cantrip_api::*;
use cantrip_mechanics::{effects, select, targets, triggers};
use cantrip_oracle as oracle;

/// "You may sacrifice a creature or Vehicle" (CR 701.21a): a Vehicle is a permanent that is not a creature until crewed
/// (CR 301.7), so it is named separately. This creature itself qualifies.
fn creature_or_vehicle(q: &dyn Query, choice: &Choice, _: u32) -> Verdict {
    Verdict::each(choice, |card| targets::creature_or_vehicle(q, card))
}

/// "When this creature enters, you may sacrifice a creature or Vehicle. When you do, ..." (CR 603.12).
fn sacrifice_to_destroy(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let rule = Selection::new(select::up_to(1))
        .with(select::part(&(creature_or_vehicle as SelectFn), 0, ""))
        .called("creature or Vehicle");
    if !effects::may_sacrifice(g, item.controller, item.object, rule)?.is_empty() {
        let this = g.card_id(&CARD);
        g.reflexive_trigger(item.object, item.controller, AbilityRef::delayed(this, 0), Target::None);
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(triggers::THIS_ENTERS, sacrifice_to_destroy,
        "When this creature enters, you may sacrifice a creature or Vehicle. When you do, destroy target artifact an opponent controls.")],
    delayed: &[delayed_trigger(effects::destroy_first_target, "When you do, destroy target artifact an opponent controls.")
        .targets(&[targets::ARTIFACT_AN_OPPONENT_CONTROLS])],
    ..oracle::GASTAL_BLOCKBUSTER
};
