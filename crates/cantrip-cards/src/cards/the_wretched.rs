use cantrip_api::*;
use cantrip_mechanics::triggers;
use cantrip_oracle as oracle;

/// "At end of combat, gain control of all creatures blocking this creature for as long as you control this creature." (CR 608.2, 611.2a).
fn gain_control_all_creatures_blocking(
    g: &mut dyn Game,
    item: &StackItem,
) -> Result<(), Suspend> {
    let objects = {
        let q: &dyn Query = g;
        q.creatures_blocking(item.object)
            .into_iter()
            .filter(|&id| q.chars(id).is(CardType::Creature))
            .collect::<Vec<ObjectId>>()
    };
    let duration = Duration::WhileSourceControlled;
    instruction(g, |g| {
        for &object in &objects {
            g.add_effect(item.object, object, EffectKind::Control(item.controller), duration);
        }
    });
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        triggers::END_OF_COMBAT,
        gain_control_all_creatures_blocking,
        "At end of combat, gain control of all creatures blocking this creature for as long as you control this creature.",
    )],
    ..oracle::THE_WRETCHED
};
