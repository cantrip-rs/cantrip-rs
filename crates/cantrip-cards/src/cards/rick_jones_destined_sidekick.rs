use cantrip_api::*;
use cantrip_mechanics::library;
use cantrip_oracle as oracle;

/// "Mill four cards. You may put a Hero or enchantment card from among those cards into your hand." (CR 701.17, 701.17c): the
/// cards milled are the ones it may take from, as they are where they went; Hero is a creature type of this card's text (CR 612).
/// Nobody is asked when none of them is a Hero or an enchantment card, and choosing none declines.
fn mill_four_and_take_a_hero_or_enchantment(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let hero = g.words(item.object).subtype(Subtype::Hero);
    library::mill_then_may_take_into_hand(g, item.controller, item.object, 4, "Hero or enchantment", |c| {
        c.has_subtype(hero) || c.is(CardType::Enchantment)
    })?;
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[ability(
        cost("{3}, {T}"),
        mill_four_and_take_a_hero_or_enchantment,
        "{3}, {T}: Mill four cards. You may put a Hero or enchantment card from among those cards into your hand.",
    )],
    ..oracle::RICK_JONES_DESTINED_SIDEKICK
};
