use cantrip_api::*;
use cantrip_oracle as oracle;

/// "{T}: Add two mana in any combination of colors. Spend this mana only to cast Lesson spells." Each mana is a color chosen
/// as the ability resolves, repeats allowed (CR 106.1b), and pays only for casting a spell with the subtype Lesson
/// (CR 106.6, 205.3).
fn add_two_for_lessons(g: &mut dyn Game, asker: &Asker, _: Option<Color>) -> Result<(), Suspend> {
    for color in cantrip_mechanics::choose_colors(g, asker.controller, asker.source, ColorSet(0b11111), 2)? {
        g.add_restricted_mana(asker.controller, color.mana(), 1, ManaRestriction::Subtypes(SubtypeSpells::of(Subtype::Lesson)));
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        mana_ability(TAP, add_any_color(1), "{T}: Add one mana of any color."),
        mana_ability(
            TAP,
            add_by(add_two_for_lessons).yields_any(2).only_for(ManaRestriction::Subtypes(SubtypeSpells::of(Subtype::Lesson))),
            "{T}: Add two mana in any combination of colors. Spend this mana only to cast Lesson spells.",
        ),
    ],
    ..oracle::HERMITIC_HERBALIST
};
