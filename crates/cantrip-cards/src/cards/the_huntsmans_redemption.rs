use cantrip_api::select::{Choice, SelectFn, Verdict};
use cantrip_api::*;
use cantrip_mechanics::{effects, lands, library, select, targets, tokens};
use cantrip_oracle as oracle;

/// "A creature card or a basic land card" (CR 205.4a, 205.4c): either kind, as the card is in the library.
fn creature_or_basic_land(q: &dyn Query, choice: &Choice, _: u32) -> Verdict {
    Verdict::each(choice, |id| q.chars(id).is(CardType::Creature) || lands::is_basic_land_card(q, id))
}

/// "II — You may sacrifice a creature. If you do, search your library for a creature or basic land card, reveal it, put it into
/// your hand, then shuffle." (CR 701.21a, 701.23, 701.24): declining is the empty answer to an up-to-one choice, and only a
/// sacrifice that happened (CR 608.2c) lets the search happen. The card may be left unfound (CR 701.23b); the library is
/// shuffled whenever it is searched.
fn sacrifice_a_creature_to_search(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let you = item.controller;
    if effects::may_sacrifice(g, you, item.object, select::creature_permanents(select::up_to(1)))?.is_empty() {
        return Ok(());
    }
    let rule = select::cards(select::up_to(1)).with(select::part(&(creature_or_basic_land as SelectFn), 0, "")).called("creature or basic land");
    library::find_to_hand(g, you, item.object, rule, true)?;
    Ok(())
}

/// "III — Up to two target creatures each get +2/+2 and gain trample until end of turn." (CR 611.2a)
fn two_creatures_get_plus_two_and_trample(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    effects::targets_get_and_gain(g, item, 2, 2, Keyword::Trample)
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        chapter(1, tokens::create_a_3_3_beast, "I \u{2014} Create a 3/3 green Beast creature token."),
        chapter(
            2,
            sacrifice_a_creature_to_search,
            "II \u{2014} You may sacrifice a creature. If you do, search your library for a creature or basic land card, reveal it, put it into your hand, then shuffle.",
        ),
        chapter(3, two_creatures_get_plus_two_and_trample, "III \u{2014} Up to two target creatures each get +2/+2 and gain trample until end of turn.")
            .targets(&[targets::CREATURE.between(0, 2)]),
    ],
    ..oracle::THE_HUNTSMANS_REDEMPTION
};

pub static EXTRA: &[&CardDef] = &[&tokens::BEAST_3_3];
