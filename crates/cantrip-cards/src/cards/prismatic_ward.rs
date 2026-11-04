use cantrip_api::*;
use cantrip_mechanics::{choose_color, targets};
use cantrip_oracle as oracle;

/// "As this Aura enters, choose a color. Prevent all damage that would be dealt to enchanted creature by sources of the chosen color." (CR 614.1c).
fn as_aura_enters_choose_color_enters(
    g: &mut dyn Game,
    entering: &mut Entering,
) -> Result<(), Suspend> {
    entering.chosen_color = Some(choose_color(g, entering.controller, entering.object)?);
    Ok(())
}

/// "As this Aura enters, choose a color. Prevent all damage that would be dealt to enchanted creature by sources of the chosen color." (CR 615.1).
fn as_aura_enters_choose_color_applies(
    q: &dyn Query,
    asker: &Asker,
    e: &DamageEvent,
) -> bool {
    {
        let (source, target) = (e.source, &e.target);
        if q
            .state()
            .chosen_color(q.linked_to(asker))
            .is_some_and(|chosen| q.chars(source).colors.has(chosen))
            && matches!(
                * target, Target::Object(o) if Some(asker.source).and_then(| o : ObjectId
                | q.state().obj(o).attached_to) == Some(o)
            )
        {
            return true;
        }
    }
    false
}

pub static CARD: CardDef = CardDef {
    preventions: &[
        PreventionDef {
            hint: "prevents all damage that would be dealt to enchanted creature by sources of the chosen color",
            applies: as_aura_enters_choose_color_applies,
            amount: PreventionAmount::All,
        },
    ],
    enters: Some(as_aura_enters_choose_color_enters),
    enchant: Some(targets::CREATURE),
    ..oracle::PRISMATIC_WARD
};
