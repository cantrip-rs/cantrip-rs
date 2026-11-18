use cantrip_api::*;
use cantrip_mechanics::{disguise, triggers};
use cantrip_oracle as oracle;

/// "When this land is turned face up, add two mana of any one color. Until end of turn, you don't lose this mana as steps
/// and phases end" (CR 106.3, 106.4, 514.2): the one color is chosen, then both mana are added to the pool as mana that
/// stays until the turn's end.
fn add_two_mana_of_any_one_color_until_end_of_turn(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let color = cantrip_mechanics::choose_color_for(g, item.controller, item.object, ColorUse::AddMana)?;
    g.add_mana_until_end_of_turn(item.controller, color.mana(), 2);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        mana_ability(TAP, add(Mana::C), "{T}: Add {C}."),
        triggered(
            triggers::THIS_TURNED_FACE_UP,
            add_two_mana_of_any_one_color_until_end_of_turn,
            "When this land is turned face up, add two mana of any one color. Until end of turn, you don't lose this mana as steps and phases end.",
        ),
    ],
    ways: &[disguise::way(Cost::mana(mana("{3}")), "Disguise {3}")],
    ..oracle::BRANCH_OF_VITU_GHAZI
};
