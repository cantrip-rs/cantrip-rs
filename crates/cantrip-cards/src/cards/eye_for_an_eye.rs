use cantrip_api::*;
use cantrip_mechanics::prevention;
use cantrip_oracle as oracle;

/// The original damage remains and Eye for an Eye deals the same amount simultaneously (CR 614.1a).
fn reflected_damage(q: &dyn Query, asker: &Asker, _: ObjectId, before: &DamageEvent, _: &DamageEvent) -> Vec<DamageComponent> {
    vec![DamageComponent::Additional(DamageEvent {
        source: asker.source,
        target: Target::Player(q.controller(before.source)),
        amount: before.amount,
        combat: false,
        heal_marked: false,
        shielded: None,
    })]
}

fn applies(q: &dyn Query, asker: &Asker, chosen: ObjectId, proposal: &Proposal) -> bool {
    matches!(proposal, Proposal::Damage(d) if d.target == Target::Player(asker.controller)
        && prevention::by_chosen_source(q, chosen, d))
}

/// The replacement leaves the original damage in place; `reflected_damage` adds the other component.
fn retain_damage(_: &dyn Query, _: &Asker, _: ObjectId, _: &mut Proposal) {}

/// Choose a damage source and watch it for the rest of this turn (CR 609.7a, 611.2a).
fn eye(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(source) = prevention::choose_source(g, item, prevention::SOURCE)? else { return Ok(()) };
    g.end_sentence();
    g.add_effect(item.object, source, EffectKind::Replacement { card: item.ability.card, index: 0 }, Duration::EndOfTurn);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[], eye),
    granted_replacements: &[granted_replacement(ReplacementKind::Damage, applies, retain_damage, "")
        .with_damage(reflected_damage)
        .once()
        .text("The next time a source of your choice would deal damage to you this turn, instead that source deals that much damage to you and Eye for an Eye deals that much damage to that source's controller.")],
    ..oracle::EYE_FOR_AN_EYE
};
