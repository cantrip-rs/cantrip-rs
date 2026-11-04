use cantrip_api::*;
use cantrip_mechanics::choose;
use cantrip_oracle as oracle;

/// Choose which enchantments to destroy as the spell resolves (CR 608.2d, 701.8a).
fn destroy_enchantments(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let white = g.words(item.object).color(Color::White);
    let chosen = choose::one_of(g, item.controller, item.object, vec![
        Label::Card(item.ability.card, 5),
        Label::Card(item.ability.card, 0),
    ])?;
    let objects: Vec<_> = g.state().battlefield().iter().copied().filter(|&id| {
        let chars = g.chars(id);
        chars.is(CardType::Enchantment) && (chosen == 0 || !chars.colors.has(white))
    }).collect();
    g.destroy_all(&objects);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[], destroy_enchantments),
    labels: &[
        "Destroy all nonwhite enchantments",
        "Destroy all nonblue enchantments",
        "Destroy all nonblack enchantments",
        "Destroy all nonred enchantments",
        "Destroy all nongreen enchantments",
        "Destroy all enchantments",
    ],
    ..oracle::ESSENCE_FILTER
};
