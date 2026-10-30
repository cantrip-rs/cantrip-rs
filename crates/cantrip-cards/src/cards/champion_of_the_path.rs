use cantrip_api::*;
use cantrip_mechanics::triggers::{another_of_subtype_you_control_enters, THIS_LEAVES};
use cantrip_mechanics::{behold, effects, lands, targets};
use cantrip_oracle as oracle;

/// "Whenever another Elemental you control enters, it deals damage equal to its power to each opponent" (CR 603.6a, 120.3): the
/// Elemental that entered is the source (`StackItem::subject`), at its power as it last existed if it has left (CR 608.2h,
/// 113.7a); one that is no longer a creature has none, so nothing is dealt. The damage is dealt to the opponents together.
fn elemental_deals_damage_to_each_opponent(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(elemental) = item.subject.object() else { return Ok(()) };
    let power = targets::power_or_last_known(g, elemental).max(0) as u16;
    let hits: Vec<(ObjectId, Target, u16)> = g.state().opponents(item.controller).map(|p| (elemental, Target::Player(p), power)).collect();
    g.damage_together(&hits);
    Ok(())
}

/// "As an additional cost to cast this spell, behold an Elemental and exile it" (CR 601.2b, 701.4a), exiled with the creature the
/// spell becomes (CR 607.2q), which returns it when it leaves the battlefield. A spell that is countered leaves it exiled.
pub static CARD: CardDef = CardDef {
    additional_cost: Some(behold::cost_and_exile(behold::elemental, "Elemental")),
    abilities: &[
        triggered(
            when_asked(EventMask::ZONE_CHANGE, another_of_subtype_you_control_enters::<{ lands::sub(Subtype::Elemental) }>),
            elemental_deals_damage_to_each_opponent,
            "Whenever another Elemental you control enters, it deals damage equal to its power to each opponent.",
        ),
        triggered(THIS_LEAVES, effects::return_exiled_to_hand, "When this creature leaves the battlefield, return the exiled card to its owner's hand."),
    ],
    ..oracle::CHAMPION_OF_THE_PATH
};
