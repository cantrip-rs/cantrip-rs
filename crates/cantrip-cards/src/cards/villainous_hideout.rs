use cantrip_api::*;
use cantrip_mechanics::targets;
use cantrip_oracle as oracle;

/// "Target Villain you control" (CR 115.1): a Villain permanent its controller controls, "Villain" read through this land's text
/// (CR 612).
fn villain_you_control(q: &dyn Query, asker: &Asker, t: Target) -> bool {
    matches!(t, Target::Object(id)
        if targets::permanent(q, id) && q.controller(id) == asker.controller && q.chars(id).has_subtype(q.words(asker.source).subtype(Subtype::Villain)))
}

/// "Target Villain you control connives" (CR 701.50): the target is still the creature it was, if it is still legal (CR 608.2b).
fn connive_villain(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    match targets::object(g, item, 0) {
        Some(villain) => cantrip_mechanics::connive(g, villain, item.object),
        None => Ok(()),
    }
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        mana_ability(TAP, add(Mana::C), "{T}: Add {C}."),
        mana_ability(
            TAP,
            add_any_color(1).only_for(ManaRestriction::Subtypes(SubtypeSpells::of(Subtype::Villain).or_abilities())),
            "{T}: Add one mana of any color. Spend this mana only to cast a Villain spell or to activate an ability of a Villain source.",
        ),
        ability(
            cost("{3}, {T}"),
            connive_villain,
            "{3}, {T}: Target Villain you control connives. Activate only as a sorcery.",
        )
        .targets(&[target(villain_you_control)])
        .sorcery_speed(),
    ],
    ..oracle::VILLAINOUS_HIDEOUT
};
