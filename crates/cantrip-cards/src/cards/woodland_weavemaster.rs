use cantrip_api::*;
use cantrip_mechanics::{effects, lands, triggers};
use cantrip_oracle as oracle;

/// "Add X mana of any one color, where X is this creature's power. Spend this mana only to cast Elf spells and activate abilities
/// of Elf sources." (CR 106.5, 106.6, 605.3b): the power as the ability is activated, at once; none for a power of 0 or less. The
/// color is always named, the ability offering all five. The mana pays only for casting a spell that is an Elf on the stack or for
/// activating an ability of a source that is an Elf, a permanent or a card elsewhere (CR 106.6); "Elf" is a word of its text, read
/// through its words (CR 612).
fn power_in_mana(g: &mut dyn Game, asker: &Asker, color: Option<Color>) -> Result<(), Suspend> {
    let power = g.chars(asker.source).power.unwrap_or(0).max(0) as u32;
    let elf = g.words(asker.source).subtype(Subtype::Elf);
    g.add_restricted_mana(
        asker.controller,
        color.unwrap_or(Color::White).mana(),
        power,
        ManaRestriction::Subtypes(SubtypeSpells::of(elf).or_abilities()),
    );
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            when_asked(EventMask::ZONE_CHANGE, triggers::another_of_subtype_you_control_enters::<{ lands::sub(Subtype::Elf) }>),
            effects::this_gets::<1, 1>,
            "Whenever another Elf you control enters, this creature gets +1/+1 until end of turn.",
        ),
        mana_ability(
            TAP,
            add_by(power_in_mana).choose_color().only_for(ManaRestriction::Subtypes(SubtypeSpells::of(Subtype::Elf).or_abilities())),
            "{T}: Add X mana of any one color, where X is this creature's power. Spend this mana only to cast Elf spells and activate abilities of Elf sources.",
        ),
    ],
    ..oracle::WOODLAND_WEAVEMASTER
};
