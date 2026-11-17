use cantrip_api::*;
use cantrip_mechanics::library::{look_at_top, Rest};
use cantrip_mechanics::triggers;
use cantrip_oracle as oracle;

/// "When this creature enters and whenever you solve a Case" (CR 603.6a, 719.3b): this creature entering, and a Case becoming solved
/// under your control (`Event::Solved`, the permanent's controller as it is solved), the Auditor itself included if it is on the
/// battlefield as the Case becomes solved.
fn this_enters_or_you_solve_a_case(q: &dyn Query, me: &Asker, event: &Event) -> Option<Fired> {
    match event {
        Event::Solved { permanent } if q.controller(*permanent) == me.controller => Fired::PLAIN,
        _ => triggers::this_enters(q, me.source, event),
    }
}

/// "Look at the top six cards of your library. You may reveal an enchantment card from among them and put it into your hand. Put the rest
/// on the bottom of your library in a random order." (CR 401.4, 701.20a): a library with fewer cards shows what it has (CR 609.3).
fn look(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    look_at_top(g, item.controller, item.object, 6, (0, 1), true, |c| c.is(CardType::Enchantment), Rest::BottomRandom)?;
    Ok(())
}

/// "You may spend mana as though it were mana of any color to cast Case spells." (CR 609.4b, 601.2h): the controller's own casting of a
/// Case spell - not an ability's activation, nor any other payment for one.
fn case_spells(q: &dyn Query, asker: &Asker, spending: &Spending) -> bool {
    spending.player == asker.controller
        && spending.purpose == SpendingFor::Casting
        && spending.source.is_some_and(|source| q.chars(source).has_subtype(Subtype::Case))
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        when_asked(EventMask(EventMask::ZONE_CHANGE.0 | EventMask::SOLVED.0), this_enters_or_you_solve_a_case),
        look,
        "When this creature enters and whenever you solve a Case, look at the top six cards of your library. You may reveal an enchantment card from among them and put it into your hand. Put the rest on the bottom of your library in a random order.",
    )],
    spend_any_color: &[case_spells],
    ..oracle::CASE_FILE_AUDITOR
};
