use cantrip_api::*;
use cantrip_mechanics::choose;
use cantrip_mechanics::triggers::THIS_ATTACKS;
use cantrip_oracle as oracle;

/// "You may tap another untapped creature you control. If you do, this creature can't be blocked this turn" (CR 701.26a, 509.1b,
/// 611.2): chosen as the ability resolves, none chosen being "you don't", and with no other untapped creature nothing is asked
/// (CR 609.3). Only a creature that really became tapped counts as "you do". The effect is on this creature, which does nothing if it
/// has left the battlefield (CR 400.7).
fn may_tap_another_to_be_unblockable(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let you = item.controller;
    let others: Vec<ObjectId> =
        g.permanents_of(you).into_iter().filter(|&id| id != item.object && !g.obj(id).tapped && g.chars(id).is(CardType::Creature)).collect();
    if others.is_empty() {
        return Ok(());
    }
    if let Some(&creature) = choose::cards(g, you, item.object, Purpose::Tap, others, 1, true)?.first() {
        g.tap(creature);
        g.add_effect(item.object, item.object, EffectKind::CantBeBlocked, Duration::EndOfTurn);
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        THIS_ATTACKS,
        may_tap_another_to_be_unblockable,
        "Whenever this creature attacks, you may tap another untapped creature you control. If you do, this creature can't be blocked this turn.",
    )],
    ..oracle::GRAVELGILL_SCOUNDREL
};
