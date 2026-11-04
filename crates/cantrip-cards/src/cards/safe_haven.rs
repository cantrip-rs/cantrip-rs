use cantrip_api::*;
use cantrip_mechanics::{effects, targets, triggers};
use cantrip_oracle as oracle;

const SACRIFICE: Question = Question::new(
    "SacrificeSafeHaven", "sacrifice this land?", "Return each card exiled with it under its owner's control", "Sacrifice", "Don't sacrifice",
);

/// Sacrifice the land if possible and chosen, then return the linked cards (CR 701.21a, 607.2a).
fn sacrifice_and_return(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if !g.can_sacrifice(item.controller, item.object)
        || !effects::ask_may(g, item.controller, item.object, Purpose::Ask(&SACRIFICE))?
        || g.sacrifice(item.controller, &[item.object]).is_empty()
    {
        return Ok(());
    }
    g.end_sentence();
    effects::return_exiled_to_battlefield(g, item)
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        ability(cost("{2}").and(TAP), effects::exile_first_target_with_this,
            "{2}, {T}: Exile target creature you control.")
            .targets(&[targets::CREATURE_YOU_CONTROL]),
        triggered(triggers::YOUR_UPKEEP, sacrifice_and_return,
            "At the beginning of your upkeep, you may sacrifice this land. If you do, return each card exiled with this land to the battlefield under its owner's control."),
    ],
    ..oracle::SAFE_HAVEN
};
