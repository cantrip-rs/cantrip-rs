use cantrip_api::*;
use cantrip_mechanics::attach::{attached_object, equip};
use cantrip_mechanics::statics::{has_delayed_ability, have_reach};
use cantrip_mechanics::{discover, effects, targets};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[equip(mana("{1}"), "Equip {1}")],
    // The abilities the equipped creature has (CR 613.1f), its own, so its controller controls and pays for them (CR 113.8).
    delayed: &[
        ability(cost("{2}").and(TAP), effects::tap_targets, "{2}, {T}: Tap target artifact or creature.").targets(&[targets::ARTIFACT_OR_CREATURE]),
        ability(
            cost("{8}").and(TAP),
            |g, item| {
                discover::discover(g, item.controller, 10)?;
                Ok(())
            },
            "{8}, {T}: Discover 10.",
        ),
    ],
    statics: &[
        static_ability(Layer::Ability, attached_object, have_reach, "Equipped creature has reach,"),
        static_ability(Layer::Ability, attached_object, has_delayed_ability::<0>, "Equipped creature has \"{2}, {T}: Tap target artifact or creature,\""),
        static_ability(Layer::Ability, attached_object, has_delayed_ability::<1>, "and \"{8}, {T}: Discover 10.\""),
    ],
    ..oracle::SWASHBUCKLERS_WHIP
};
