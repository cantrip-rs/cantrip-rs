use cantrip_api::*;
use cantrip_mechanics::statics::grant_delayed_ability;
use cantrip_mechanics::{attach, effects, targets, triggers};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        when(EventMask::ZONE_CHANGE, triggers::this_enters),
        effects::plus_one_counter_on_target,
        "When this Aura enters, put a +1/+1 counter on target creature you control.",
    )
    .targets(&[targets::CREATURE_YOU_CONTROL])],
    statics: &[static_ability(Layer::Ability, attach::attached_object, grant_delayed_ability, "Enchanted land has \"{T}: Add two mana of any one color.\"")],
    delayed: &[mana_ability(TAP, add_any_color(2), "{T}: Add two mana of any one color.")],
    enchant: Some(targets::LAND),
    ..oracle::NEW_HORIZONS
};
