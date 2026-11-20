use cantrip_api::*;
use cantrip_mechanics::targets;
use cantrip_oracle as oracle;

fn draw_a_card(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    g.draw(item.controller, 1);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[ability(
        cost("{2}").and(TAP).and(Cost::sacrifice(1, targets::filters::artifact_or_land)),
        draw_a_card,
        "{2}, {T}, Sacrifice an artifact or land: Draw a card.",
    )],
    ..oracle::MARWYN_THE_CLEARCUTTER
};
