use cantrip_api::*;
use cantrip_mechanics::{effects, sacrifice_chosen, triggers};
use cantrip_oracle as oracle;

/// "At the beginning of your upkeep, sacrifice a land. If you sacrifice an Island this way, this creature deals 3 damage to you." (CR 608.2, 120.3, 701.21a).
fn sacrifice_land_if_sacrifice_island(
    g: &mut dyn Game,
    item: &StackItem,
) -> Result<(), Suspend> {
    let sacrificed = sacrifice_chosen(g, item.controller, item.object, 1, |c| c.is(CardType::Land))?;
    g.end_sentence();
    let island = g.words(item.object).subtype(Subtype::Island);
    if sacrificed.permanents().iter().any(|&id| {
        g.state().last_known(id).is_some_and(|known| known.subtypes.has(island))
    }) {
        g.damage_together(&[(item.object, Target::Player(item.controller), 3)]);
        g.end_sentence();
    }
    Ok(())
}

/// "When you control no lands, sacrifice this creature." (CR 603.8).
fn sacrifice_creature_state(q: &dyn Query, asker: &Asker) -> bool {
    !q.state().battlefield().iter().copied().any(|id| {
        q.controller(id) == asker.controller && q.chars(id).is(CardType::Land)
    })
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            triggers::YOUR_UPKEEP,
            sacrifice_land_if_sacrifice_island,
            "At the beginning of your upkeep, sacrifice a land. If you sacrifice an Island this way, this creature deals 3 damage to you.",
        ),
        state_triggered(
            sacrifice_creature_state,
            effects::sacrifice_self,
            "When you control no lands, sacrifice this creature.",
        ),
    ],
    ..oracle::SERENDIB_DJINN
};
