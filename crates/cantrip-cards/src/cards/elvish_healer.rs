use cantrip_api::*;
use cantrip_mechanics::{prevention, targets};
use cantrip_oracle as oracle;

/// The shield's size is determined as the ability resolves (CR 615.7, 608.2h).
fn prevent_damage(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let green_creature = g.state().target(item, 0).object().is_some_and(|id| {
        let chars = g.chars(id);
        chars.is(CardType::Creature) && chars.colors.has(g.words(item.object).color(Color::Green))
    });
    prevention::next_damage_to_target(g, item, 0, 0, if green_creature { 2 } else { 1 });
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[ability(
        TAP,
        prevent_damage,
        "{T}: Prevent the next 1 damage that would be dealt to any target this turn. If it's a green creature, prevent the next 2 damage instead.",
    ).targets(&[targets::ANY])],
    granted_preventions: &[prevention::ANY_DAMAGE.text(
        "Prevent the next 1 damage that would be dealt to any target this turn. If it's a green creature, prevent the next 2 damage instead.",
    )],
    ..oracle::ELVISH_HEALER
};
