use cantrip_api::*;
use cantrip_mechanics::{mayhem, targets};
use cantrip_mechanics::triggers::THIS_ENTERS;
use cantrip_oracle as oracle;

/// Return the creature card to the battlefield under the ability's controller. It gains "This creature attacks each
/// combat if able" and the sacrifice trigger, for as long as that object stays (CR 611.2a, 400.7).
fn reanimate(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(card) = targets::object(g, item, 0) else { return Ok(()) };
    let returned = g.put_onto_battlefield(card, item.controller, false)?;
    if !g.is_on_battlefield(returned) {
        return Ok(());
    }
    // The ability Carnage's card lists, whatever card the object it resolves for has now (a merged creature's top, CR
    // 702.140e).
    let this = item.ability.card;
    // "It gains [this] and [that]": one instruction (CR 608.2c).
    instruction(g, |g| {
        g.add_effect(item.object, returned, EffectKind::MustAttack, Duration::Indefinite);
        g.add_effect(item.object, returned, EffectKind::AddAbility(AbilityRef::delayed(this, 0)), Duration::Indefinite);
    });
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(THIS_ENTERS, reanimate, "When Carnage enters, return target creature card with mana value 3 or less from your graveyard to the battlefield. It gains \"This creature attacks each combat if able\" and \"When this creature deals combat damage to a player, sacrifice it.\"")
        .targets(&[targets::CREATURE_CARD_WITH_MANA_VALUE_3_OR_LESS_IN_YOUR_GRAVEYARD])],
    ways: &[mayhem::way(mana("{B}{R}"), "Mayhem {B}{R}")],
    delayed: &[triggered(cantrip_mechanics::triggers::DEALS_COMBAT_DAMAGE_TO_PLAYER, cantrip_mechanics::effects::sacrifice_self, "When this creature deals combat damage to a player, sacrifice it.")],
    ..oracle::CARNAGE_CRIMSON_CHAOS
};
