use cantrip_api::*;
use cantrip_mechanics::triggers::ANOTHER_CREATURE_YOU_CONTROL_LEAVES_WITHOUT_DYING;
use cantrip_oracle as oracle;

fn draw(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    g.draw(item.controller, 1);
    Ok(())
}

fn return_creature(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if let Some(creature) = cantrip_mechanics::targets::object(g, item, 0) {
        g.return_to_hand(&[creature]);
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(ANOTHER_CREATURE_YOU_CONTROL_LEAVES_WITHOUT_DYING, draw,
            "Whenever one or more other creatures you control leave the battlefield without dying, draw a card.")
            .once_per_batch(),
        ability(cost("{1}{U}").and(TAP), return_creature,
            "{1}{U}, {T}: Return another target creature you control to its owner's hand.")
            .targets(&[cantrip_mechanics::targets::OTHER_CREATURE_YOU_CONTROL]),
    ],
    ..oracle::DOUR_PORT_MAGE
};
