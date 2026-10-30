use cantrip_api::*;
use cantrip_mechanics::{effects, targets, triggers};
use cantrip_oracle as oracle;

/// "If you sacrificed a Klingon this way" reads the permanents chosen for this activation's cost (CR 602.2b, 608.2h).
fn sacrifice_and_trample(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let klingon = g.cost_objects(item).iter().any(|&creature| {
        g.chars(creature)
            .has_subtype(g.words(item.object).subtype(Subtype::Klingon))
    });
    cantrip_mechanics::trample_until_end_of_turn(g, item.object, item.object);
    if klingon {
        g.draw(item.controller, 1);
        g.lose_life(item.controller, 1);
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            triggers::ANOTHER_CREATURE_DIES,
            effects::plus_one_counter_on_this,
            "Whenever another creature dies, put a +1/+1 counter on Gowron.",
        ),
        ability(
            sacrifice(1, targets::filters::another_creature),
            sacrifice_and_trample,
            "Sacrifice another creature: Gowron gains trample until end of turn. If you sacrificed a Klingon this way, you draw a card and lose 1 life.",
        ),
    ],
    ..oracle::GOWRON_EAGER_FOR_BATTLE
};
