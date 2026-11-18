use cantrip_api::*;
use cantrip_mechanics::tokens;
use cantrip_oracle as oracle;

/// "{4}, {T}: Investigate." (CR 701.16a): a Clue token for the ability's controller.
fn investigate(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    tokens::investigate(g, item.controller)
}

/// "{T}: Add {C}." is a mana ability (CR 605.1a).
pub static CARD: CardDef = CardDef {
    abilities: &[
        mana_ability(TAP, add(Mana::C), "{T}: Add {C}."),
        ability(cost("{4}, {T}"), investigate, "{4}, {T}: Investigate. (Create a Clue token. It's an artifact with \"{2}, Sacrifice this token: Draw a card.\")"),
    ],
    ..oracle::MAGNIFYING_GLASS
};
