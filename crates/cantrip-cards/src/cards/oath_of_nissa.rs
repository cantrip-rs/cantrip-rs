use cantrip_api::*;
use cantrip_mechanics::library::{look_at_top, Rest};
use cantrip_mechanics::triggers::THIS_ENTERS;
use cantrip_oracle as oracle;

/// "When Oath of Nissa enters, look at the top three cards of your library. You may reveal a creature, land, or
/// planeswalker card from among them and put it into your hand. Put the rest on the bottom of your library in any
/// order." (CR 401.4, 701.20a)
fn look(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let wanted = |c: &Chars| c.is(CardType::Creature) || c.is(CardType::Land) || c.is(CardType::Planeswalker);
    look_at_top(g, item.controller, item.object, 3, (0, 1), true, wanted, Rest::BottomChosen)?;
    Ok(())
}

/// "You may spend mana as though it were mana of any color to cast planeswalker spells." (CR 609.4b, 601.2h): the
/// controller's own casting of a planeswalker spell - not an ability's activation, nor any other payment for one.
fn planeswalker_spells(q: &dyn Query, asker: &Asker, spending: &Spending) -> AnyMana {
    if spending.player == asker.controller
        && spending.purpose == SpendingFor::Casting
        && spending.source.is_some_and(|source| q.chars(source).is(CardType::Planeswalker)) {
        AnyMana::Color
    } else {
        AnyMana::No
    }
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(THIS_ENTERS, look, "When Oath of Nissa enters, look at the top three cards of your library. You may reveal a creature, land, or planeswalker card from among them and put it into your hand. Put the rest on the bottom of your library in any order.")],
    spend_any_mana: &[planeswalker_spells],
    ..oracle::OATH_OF_NISSA
};
