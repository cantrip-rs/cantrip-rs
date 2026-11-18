use cantrip_api::*;
use cantrip_mechanics::{
    attach::{attached_object, attached_to_it, enchanted_creature_gains_until_end_of_turn},
    statics::plus_pt,
    targets,
    triggers::THIS_ENTERS,
};
use cantrip_oracle as oracle;

/// "Untap enchanted creature. It gains hexproof until end of turn. If it's suspected, it's no longer suspected." (CR 701.26b,
/// 611.2a, 701.60a): the creature the Aura is attached to as the trigger resolves, or, if the Aura has left the battlefield by
/// then, the one it was attached to as it left (CR 608.2h); a creature that has left does nothing (CR 400.7). A creature that
/// is not suspected is unchanged.
fn untap_shield_and_clear(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(enchanted) = g.obj(item.object).attached_to else { return Ok(()) };
    g.untap(enchanted);
    enchanted_creature_gains_until_end_of_turn(g, item, Keyword::Hexproof);
    g.end_sentence();
    g.unsuspect(enchanted);
    Ok(())
}

/// Flash is a core keyword.
pub static CARD: CardDef = CardDef {
    enchant: Some(targets::CREATURE),
    abilities: &[triggered(
        THIS_ENTERS,
        untap_shield_and_clear,
        "When this Aura enters, untap enchanted creature. It gains hexproof until end of turn. If it's suspected, it's no longer suspected.",
    )],
    statics: &[static_ability(Layer::PtModify, attached_object, plus_pt::<2, 2>, "Enchanted creature gets +2/+2.")],
    // "Enchanted creature ... can't become suspected" (CR 701.60, 101.2): a rule of the Aura's, not an ability of the creature
    // (CR 113.1a), so it holds whatever the creature can do.
    suspect_bans: &[attached_to_it],
    ..oracle::AIRTIGHT_ALIBI
};
