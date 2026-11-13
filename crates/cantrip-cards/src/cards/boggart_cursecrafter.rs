use cantrip_api::*;
use cantrip_mechanics::{effects, lands, triggers};
use cantrip_oracle as oracle;

/// "This creature deals 1 damage to each opponent."
fn ping(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    effects::damage_each_opponent(g, item, 1);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        when_asked(EventMask::ZONE_CHANGE, triggers::another_of_subtype_you_control_dies::<{ lands::sub(Subtype::Goblin) }>),
        ping,
        "Whenever another Goblin you control dies, this creature deals 1 damage to each opponent.",
    )],
    ..oracle::BOGGART_CURSECRAFTER
};
