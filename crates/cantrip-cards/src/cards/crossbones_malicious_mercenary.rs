use cantrip_api::*;
use cantrip_mechanics::{effects, lands, triggers};
use cantrip_oracle as oracle;

/// "Put a +1/+1 counter on Crossbones. He deals 2 damage to each opponent." (CR 122.1a, 120.3): the counter goes on him, and he
/// is the damage's source, if he is still there to put it; the damage is dealt either way (CR 608.2g, 113.7a).
fn grow_and_hurt(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    effects::plus_one_counter_on_this(g, item)?;
    g.end_sentence();
    effects::damage_each_opponent(g, item, 2);
    Ok(())
}

/// Deathtouch is a core keyword read from its characteristics.
pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        when_asked(EventMask::ZONE_CHANGE, triggers::another_of_subtype_you_control_enters::<{ lands::sub(Subtype::Villain) }>),
        grow_and_hurt,
        "Whenever another Villain you control enters, put a +1/+1 counter on Crossbones. He deals 2 damage to each opponent. This ability triggers only once each turn.",
    )
    .triggers_each_turn(1)],
    ..oracle::CROSSBONES_MALICIOUS_MERCENARY
};
