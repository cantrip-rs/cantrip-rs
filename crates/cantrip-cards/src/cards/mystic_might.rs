use cantrip_api::*;
use cantrip_mechanics::{attach, cumulative_upkeep, effects, statics, targets};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[
        cumulative_upkeep!(
            cumulative_upkeep::Payment::Mana(mana("{1}{U}")), "Cumulative upkeep {1}{U} (At the beginning of your upkeep, put an age counter on this permanent, then sacrifice it unless you pay its upkeep cost for each age counter on it.)"
        ),
    ],
    statics: &[
        static_ability(
            Layer::Ability,
            attach::attached_object,
            statics::grant_delayed_ability,
            "Enchanted land has \"{T}: Target creature gets +2/+2 until end of turn.\"",
        ),
    ],
    delayed: &[
        ability(TAP, effects::target_creature_gets::<2, 2>, "{T}: Target creature gets +2/+2 until end of turn.")
            .targets(&[targets::CREATURE]),
    ],
    enchant: Some(targets::LAND_YOU_CONTROL),
    ..oracle::MYSTIC_MIGHT
};
