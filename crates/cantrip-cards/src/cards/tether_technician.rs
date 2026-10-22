use cantrip_api::*;
use cantrip_mechanics::{damage_target, effects, targets, triggers::THIS_ENTERS};
use cantrip_oracle as oracle;

/// "When this creature enters, you may discard a card. When you do, ..." (CR 701.9, 603.12):
/// the optional discard happens on resolution; only then is the reflexive damage ability created.
fn discard_for_damage(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if effects::may_discard_a_card(g, item)? {
        let this = g.card_id(&CARD);
        g.reflexive_trigger(item.object, item.controller, AbilityRef::delayed(this, 0), Target::None);
    }
    Ok(())
}

/// "This creature deals 2 damage to any target" (CR 120.2b, 608.2b/h): the original source,
/// including its last-known information, deals damage only to the still-legal target.
fn deal_two(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    damage_target(g, item, 0, 2);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(THIS_ENTERS, discard_for_damage, "When this creature enters, you may discard a card. When you do, this creature deals 2 damage to any target.")],
    delayed: &[delayed_trigger(deal_two, "When you do, this creature deals 2 damage to any target.")
        .targets(&[targets::ANY])],
    ..oracle::TETHER_TECHNICIAN
};
