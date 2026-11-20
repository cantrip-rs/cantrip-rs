use cantrip_api::*;
use cantrip_mechanics::targets::filters;
use cantrip_mechanics::{effects, lands, targets};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[
        mana_ability(
            life(2),
            add_any_color(2).only_for(ManaRestriction::Subtypes(SubtypeSpells::of(Subtype::Equipment).or_equip_abilities())),
            "Pay 2 life: Add two mana of any one color. Spend this mana only to cast Equipment spells or activate equip abilities. Activate only once each turn.",
        )
        .once_each_turn(),
        ability(
            TAP.and(sacrifice(1, filters::subtype_attached_to_this::<{ lands::sub(Subtype::Equipment) }>)),
            effects::target_creature_gets::<-4, -4>,
            "{T}, Sacrifice an Equipment attached to Ronin: Target creature gets -4/-4 until end of turn. Activate only as a sorcery.",
        )
        .targets(&[targets::CREATURE])
        .sorcery_speed(),
    ],
    ..oracle::RONIN_SHADOW_STALKER
};
