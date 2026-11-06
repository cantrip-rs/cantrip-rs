use cantrip_api::*;
use cantrip_mechanics::{attach, targets};
use cantrip_oracle as oracle;

/// "{T}: Target creature gets +2/+1 until end of turn." (CR 608.2, 611.2a).
fn granted_0(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(target) = targets::object(g, item, 0) else { return Ok(()) };
    instruction(
        g,
        |g| {
            cantrip_mechanics::pump(g, item.object, target, 2, 1);
        },
    );
    Ok(())
}

pub static CARD: CardDef = CardDef {
    statics: &[
        static_ability(
            Layer::Ability,
            attach::attached_object,
            cantrip_mechanics::statics::grant_delayed_ability,
            "Enchanted land has \"{T}: Target creature gets +2/+1 until end of turn.\"",
        ),
    ],
    delayed: &[
        ability(TAP, granted_0, "{T}: Target creature gets +2/+1 until end of turn.")
            .targets(&[targets::CREATURE]),
    ],
    enchant: Some(targets::LAND),
    ..oracle::FORBIDDEN_LORE
};
