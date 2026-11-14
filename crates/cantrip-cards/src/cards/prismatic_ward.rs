use cantrip_api::*;
use cantrip_mechanics::{choose_color, prevention, targets};
use cantrip_oracle as oracle;

/// "As this Aura enters, choose a color. Prevent all damage that would be dealt to enchanted creature by sources of the chosen color." (CR 614.1c).
fn as_aura_enters_choose_color_enters(
    g: &mut dyn Game,
    entering: &mut Entering,
) -> Result<(), Suspend> {
    entering.chosen_color = Some(choose_color(g, entering.controller, entering.object)?);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    preventions: &[
        PreventionDef {
            hint: "prevents all damage that would be dealt to enchanted creature by sources of the chosen color",
            applies: prevention::enchanted_creature_from_sources_of_chosen_color,
            amount: PreventionAmount::All,
            then: None,
        },
    ],
    enters: Some(as_aura_enters_choose_color_enters),
    enchant: Some(targets::CREATURE),
    ..oracle::PRISMATIC_WARD
};
