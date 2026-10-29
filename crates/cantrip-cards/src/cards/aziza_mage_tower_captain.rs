use cantrip_api::*;
use cantrip_mechanics::triggers::INSTANT_OR_SORCERY_YOU_CAST;
use cantrip_mechanics::{choose, copy, effects, select};
use cantrip_oracle as oracle;

/// "You may tap three untapped creatures you control. If you do, copy that spell. You may choose new targets for the copy" (CR
/// 608.2c-d, 707.10): the tapping is chosen as the ability resolves, not paid as a cost, so summoning-sick creatures and Aziza
/// herself may be tapped for it; the copy (not cast) is made only if three were tapped, from the spell's last-known information if
/// it has left the stack (CR 608.2h). With fewer than three untapped creatures nothing is asked.
fn tap_three_to_copy(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let you = item.controller;
    let untapped: Vec<ObjectId> =
        g.permanents_of(you).into_iter().filter(|&id| g.chars(id).is(CardType::Creature) && !g.obj(id).tapped).collect();
    if untapped.len() < 3 || !effects::ask_may(g, you, item.object, Purpose::TapToPay)? {
        return Ok(());
    }
    for creature in choose::select(g, you, item.object, Purpose::Tap, untapped, select::creature_cards(select::exactly(3)))? {
        g.tap(creature);
    }
    copy::copy_the_cast_spell(g, item)
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        INSTANT_OR_SORCERY_YOU_CAST,
        tap_three_to_copy,
        "Whenever you cast an instant or sorcery spell, you may tap three untapped creatures you control. If you do, copy that spell. You may choose new targets for the copy.",
    )],
    ..oracle::AZIZA_MAGE_TOWER_CAPTAIN
};
