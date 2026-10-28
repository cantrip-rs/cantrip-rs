use cantrip_api::*;
use cantrip_mechanics::effects;
use cantrip_mechanics::{counter_unless_pays, lands, targets, triggers};
use cantrip_oracle as oracle;

/// "Sacrifice this creature: Counter target instant or sorcery spell unless its controller pays {X}, where X is this
/// creature's power." The sacrifice is a cost (CR 601.2h), so the creature is gone when this resolves and X is its power
/// as it last existed on the battlefield (CR 113.7a, 608.2h); the payment is asked even for {0} (CR 118.5).
fn counter(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let power = g.chars(item.object).power.unwrap_or(0).max(0) as u8;
    counter_unless_pays(g, item, 0, generic(power))?;
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            when_asked(EventMask::ZONE_CHANGE, triggers::another_of_subtype_you_control_enters::<{ lands::sub(Subtype::Spirit) }>),
            effects::this_gets::<1, 1>,
            "Whenever another Spirit you control enters, this creature gets +1/+1 until end of turn.",
        ),
        ability(SACRIFICE_THIS, counter, "Sacrifice this creature: Counter target instant or sorcery spell unless its controller pays {X}, where X is this creature's power.")
            .targets(&[targets::INSTANT_OR_SORCERY_SPELL]),
    ],
    ..oracle::MAUSOLEUM_WANDERER
};
