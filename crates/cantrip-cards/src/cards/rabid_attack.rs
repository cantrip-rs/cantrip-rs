use cantrip_api::*;
use cantrip_mechanics::{effects, targets, triggers};
use cantrip_oracle as oracle;

/// "Until end of turn, any number of target creatures you control each get +1/+0 and gain 'When this creature dies, draw a
/// card.'" (CR 611.2a, 613.1c, 613.1f): each creature still a legal target as this resolves (CR 608.2b) gets both, for the
/// rest of the turn; the granted ability belongs to the creature that got it, so the draw is its controller's as it last
/// existed (CR 603.10a).
fn rabid_attack(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let this = g.card_id(&CARD);
    let creatures = targets::legal_objects(g, item);
    instruction(g, |g| {
        for creature in creatures {
            cantrip_mechanics::pump(g, item.object, creature, 1, 0);
            g.add_effect(item.object, creature, EffectKind::AddAbility(AbilityRef::delayed(this, 0)), Duration::EndOfTurn);
        }
    });
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[targets::CREATURE_YOU_CONTROL.between(0, TargetSpec::ANY_NUMBER)], rabid_attack),
    // The granted ability (the card's delayed ability 0).
    delayed: &[triggered(triggers::THIS_DIES, |g, item| effects::draw(g, item, 1), "When this creature dies, draw a card.")],
    ..oracle::RABID_ATTACK
};
