use cantrip_api::*;
use cantrip_mechanics::targets;
use cantrip_oracle as oracle;

/// Count the target opponent's creatures of the color named by this spell as it resolves (CR 608.2h, 612.1).
fn damage(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(opponent) = targets::player(g, item, 0) else { return Ok(()) };
    let color = g.words(item.object).color(Color::White);
    let count = g.state().battlefield().iter().copied().filter(|&id| {
        let chars = g.chars(id);
        g.controller(id) == opponent && chars.is(CardType::Creature) && chars.colors.has(color)
    }).count();
    let amount = i64::try_from(count).expect("creature count fits the native object domain") * 2;
    g.damage(item.object, Target::Player(opponent), amount);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[targets::OPPONENT], damage),
    ..oracle::JOVIAL_EVIL
};
