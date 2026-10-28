use cantrip_api::*;
use cantrip_mechanics::effects;
use cantrip_oracle as oracle;

/// "Add four mana in any combination of colors." Each mana is a color chosen as the ability resolves, repeats allowed
/// (CR 106.1b).
fn add_four(g: &mut dyn Game, asker: &Asker, _: Option<Color>) -> Result<(), Suspend> {
    for color in cantrip_mechanics::choose_colors(g, asker.controller, asker.source, ColorSet(0b11111), 4)? {
        g.add_mana(asker.controller, color.mana(), 1);
    }
    Ok(())
}

/// "Activate only if you control a creature with toughness 4 or greater" (CR 602.5b): checked as the ability is activated,
/// before its cost is paid, with the toughness the creature has now.
fn control_a_creature_with_toughness_four_or_greater(q: &dyn Query, asker: &Asker) -> bool {
    q.permanents_of(asker.controller).into_iter().any(|id| {
        let c = q.chars(id);
        c.is(CardType::Creature) && c.toughness.is_some_and(|t| t >= 4)
    })
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        mana_ability(TAP, add(Mana::C), "{T}: Add {C}."),
        mana_ability(cost("{4}, {T}"), add_by(add_four).yields_any(4), "{4}, {T}: Add four mana in any combination of colors."),
        ability(
            cost("{4}, {T}"),
            |g, item| effects::draw(g, item, 1),
            "{4}, {T}: Draw a card. Activate only if you control a creature with toughness 4 or greater.",
        )
        .condition(control_a_creature_with_toughness_four_or_greater),
    ],
    ..oracle::BAXTER_BUILDING
};
