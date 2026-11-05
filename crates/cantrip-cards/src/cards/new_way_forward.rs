use cantrip_api::*;
use cantrip_mechanics::prevention;
use cantrip_oracle as oracle;

/// "When damage is prevented this way" (CR 603.12, 615.5): a reflexive ability, put on the stack once the damage is prevented,
/// counting how much was (CR 603.12: "that much"), about the controller of the object that would have dealt it, as it last
/// existed (CR 608.2h). It is controlled by the spell's controller and the spell is its source.
fn when_prevented(g: &mut dyn Game, asker: &Asker, _chosen: ObjectId, prevented: &[Prevented]) {
    let Some(first) = prevented.first() else { return };
    let to = g.controller(first.source);
    let amount = Prevented::total(prevented);
    let this = g.card_id(&CARD);
    g.reflexive_trigger_counted(asker.source, asker.controller, AbilityRef::delayed(this, 0), Target::Player(to), amount);
}

/// "New Way Forward deals that much damage to that source's controller and you draw that many cards": the damage is dealt by the
/// spell, as it last existed (CR 608.2h), to the player the ability is about, and then its controller draws, in that order.
fn deal_and_draw(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    g.damage(item.object, item.subject, item.x);
    g.draw(item.controller, item.x);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    // "The next time a source of your choice would deal damage to you this turn, prevent that damage" (CR 609.7a, 615.8): the
    // source is chosen as the spell resolves; it is not targeted.
    spell: spell(&[], |g, item| {
        prevention::next_time_chosen_source_would_damage_you(g, item, 0)?;
        Ok(())
    }),
    granted_preventions: &[GrantedPreventionDef {
        then: Some(when_prevented),
        text: "The next time a source of your choice would deal damage to you this turn, prevent that damage. When damage is prevented this way, New Way Forward deals that much damage to that source's controller and you draw that many cards.",
        ..prevention::FROM_CHOSEN_SOURCE
    }],
    delayed: &[delayed_trigger(deal_and_draw, "When damage is prevented this way, New Way Forward deals that much damage to that source's controller and you draw that many cards.")],
    ..oracle::NEW_WAY_FORWARD
};
