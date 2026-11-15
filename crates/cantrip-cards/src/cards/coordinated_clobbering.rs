use cantrip_api::*;
use cantrip_mechanics::{fight, targets};
use cantrip_oracle as oracle;

/// "Tap one or two target untapped creatures you control. They each deal damage equal to their power to target creature an
/// opponent controls." Each target is judged as the spell begins to resolve (CR 608.2b), before anything is tapped, so a creature
/// that was an illegal target then taps nothing and deals nothing, and the ones tapped here still deal damage though they are no
/// longer untapped (ruling 2024-09-20). The tapping is one instruction (CR 701.26a, `tap_together`); with the recipient illegal the sources are
/// still tapped and no damage is dealt.
fn tap_then_deal_damage(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let recipient_slot = item.target_count as usize - 1;
    let sources: Vec<ObjectId> = (0..recipient_slot).filter_map(|slot| targets::object(g, item, slot)).collect();
    let recipient = targets::object(g, item, recipient_slot);
    g.tap_together(&sources);
    if let Some(recipient) = recipient {
        let hits: Vec<(ObjectId, Target, u32)> =
            sources.iter().map(|&source| (source, Target::Object(recipient), fight::damage_equal_to_power(g, source))).collect();
        g.damage_together(&hits);
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(
        &[
            targets::UNTAPPED_CREATURE_YOU_CONTROL.between(1, 2),
            targets::CREATURE_AN_OPPONENT_CONTROLS,
        ],
        tap_then_deal_damage,
    ),
    ..oracle::COORDINATED_CLOBBERING
};
