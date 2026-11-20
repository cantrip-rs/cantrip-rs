use cantrip_api::*;
use cantrip_mechanics::{damage_target, statics, targets};
use cantrip_oracle as oracle;

const NAME: &str = "Food Fight";

/// "It deals damage to any target equal to 1 plus the number of permanents named Food Fight you control" (CR 201.2, 608.2h): the
/// artifact is the source, "you" is its controller, and the count is made as the ability resolves, the Food Fights that are on
/// the battlefield then - this ability's own artifact, sacrificed to pay for it, no longer among them if it was one.
fn damage_for_food_fights(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let state = g.state();
    let food_fights = state.battlefield().iter().filter(|&&id| state.obj(id).controller == item.controller && g.chars(id).name == NAME).count();
    damage_target(g, item, 0, 1 + food_fights as u32);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    delayed: &[ability(
        cost("{2}").and(SACRIFICE_THIS),
        damage_for_food_fights,
        "{2}, Sacrifice this artifact: It deals damage to any target equal to 1 plus the number of permanents named Food Fight you control.",
    )
    .targets(&[targets::ANY])],
    statics: &[static_ability(
        Layer::Ability,
        statics::your_artifacts,
        statics::grant_delayed_ability,
        "Artifacts you control have \"{2}, Sacrifice this artifact: It deals damage to any target equal to 1 plus the number of permanents named Food Fight you control.\"",
    )],
    ..oracle::FOOD_FIGHT
};
