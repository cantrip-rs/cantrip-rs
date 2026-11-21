use cantrip_api::*;
use cantrip_mechanics::{effects, lands, mana, triggers};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            when_asked(EventMask::ZONE_CHANGE, triggers::another_of_subtype_you_control_enters::<{ lands::sub(Subtype::Elf) }>),
            effects::this_gets::<1, 1>,
            "Whenever another Elf you control enters, this creature gets +1/+1 until end of turn.",
        ),
        mana_ability(
            TAP,
            mana::current_power_of_one_color().only_for(ManaRestriction::Subtypes(SubtypeSpells::of(Subtype::Elf).or_abilities())),
            "{T}: Add X mana of any one color, where X is this creature's power. Spend this mana only to cast Elf spells and activate abilities of Elf sources.",
        ),
    ],
    ..oracle::WOODLAND_WEAVEMASTER
};
