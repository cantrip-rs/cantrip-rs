use cantrip_api::*;
use cantrip_mechanics::{choose, select, targets};
use cantrip_oracle as oracle;

/// I, II, III: "Mill two cards. If two cards that share a card type were milled this way, draw a card and repeat this
/// process." (CR 701.17a, 608.2c): the controller mills two, and when exactly two cards were milled (a library of one gives
/// no pair, and a replaced mill gives what really went to the graveyard) that share a card type, draws and mills two more,
/// again and again until a pair does not share one. A library of none ends it, as nothing is milled.
fn mill_and_repeat(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    loop {
        let milled = g.mill(item.controller, 2);
        if milled.len() != 2 || !select::objects_share_a_card_type(&*g, &milled) {
            return Ok(());
        }
        g.end_sentence();
        g.draw(item.controller, 1);
        g.end_sentence();
    }
}

/// "Target instant, sorcery, and/or Tamiyo planeswalker card from your graveyard" (CR 115.1, 205.3j): a card in the
/// controller's graveyard that is an instant or a sorcery or a planeswalker with the subtype Tamiyo.
fn instant_sorcery_or_tamiyo_in_your_graveyard(q: &dyn Query, asker: &Asker, t: Target) -> bool {
    targets::card_in_your_graveyard(q, asker, t, &[])
        && matches!(t, Target::Object(id) if {
            let card = q.chars(id);
            card.is(CardType::Instant) || card.is(CardType::Sorcery) || (card.is(CardType::Planeswalker) && card.has_subtype(Subtype::Tamiyo))
        })
}

/// IV: "Exile any number of target instant, sorcery, and/or Tamiyo planeswalker cards from your graveyard. Copy them. You may
/// cast any number of the copies." The targets still legal are exiled together (CR 608.2b), each copied in exile (CR 707.12),
/// and the controller may cast the copies one at a time as this resolves (CR 608.2g), whatever their timing, paying their
/// costs; a copy not cast ceases to exist (CR 707.10a, 704.5e). Nothing happens with no target chosen.
fn exile_copy_and_cast(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let cards: Vec<ObjectId> = (0..item.target_count as usize).filter_map(|slot| targets::object(g, item, slot)).collect();
    let exiled: Vec<ObjectId> = g.exile_all(&cards).into_iter().filter(|&c| g.obj(c).zone.is(ZoneKind::Exile)).collect();
    g.end_sentence();
    let copies: Vec<ObjectId> = exiled.into_iter().map(|c| g.create_copy_in_exile(c, item.controller)).collect();
    g.end_sentence();
    choose::cast_up_to_among(g, item.controller, item.object, Purpose::Cast, &copies, usize::MAX, CastTerms::default(), |_, _| true)
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        chapter(1, mill_and_repeat, "I \u{2014} Mill two cards. If two cards that share a card type were milled this way, draw a card and repeat this process."),
        chapter(2, mill_and_repeat, "II \u{2014} Mill two cards. If two cards that share a card type were milled this way, draw a card and repeat this process."),
        chapter(3, mill_and_repeat, "III \u{2014} Mill two cards. If two cards that share a card type were milled this way, draw a card and repeat this process."),
        chapter(4, exile_copy_and_cast, "IV \u{2014} Exile any number of target instant, sorcery, and/or Tamiyo planeswalker cards from your graveyard. Copy them. You may cast any number of the copies.")
            .targets(&[up_to(TargetSpec::ANY_NUMBER, instant_sorcery_or_tamiyo_in_your_graveyard)]),
    ],
    ..oracle::THE_TALE_OF_TAMIYO
};
