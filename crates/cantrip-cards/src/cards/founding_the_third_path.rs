use cantrip_api::*;
use cantrip_mechanics::{choose, copy, effects, targets};
use cantrip_oracle as oracle;

/// I: "You may cast an instant or sorcery spell with mana value 1 or 2 from your hand without paying its mana cost." The spell is
/// cast as the chapter resolves, its timing waived (CR 608.2g), judged on the face that would be cast (CR 709.3a): its mana value
/// is the spell's, so 0 and 3 are both out (CR 202.3); X is 0 and additional costs are paid (CR 118.9, 107.3b).
fn cast_from_hand(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    choose::may_cast_from_hand_free(g, item.controller, item.object, "instant or sorcery", |g, way| {
        let types = &g.card(way.face).types;
        (types.has(CardType::Instant) || types.has(CardType::Sorcery)) && (1..=2).contains(&way.mana_value)
    })
}

/// III: "Exile target instant or sorcery card from your graveyard. Copy it. You may cast the copy." The target is exiled (a new
/// object, CR 400.7) and "it" is the card in exile, copied there (CR 707.12); the copy is cast, paying its costs, as the chapter
/// resolves or not at all (CR 608.2g). With the target gone nothing is exiled or copied (CR 608.2b).
fn exile_and_copy(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(card) = targets::object(g, item, 0) else { return Ok(()) };
    let exiled = g.exile(card);
    copy::may_cast_a_copy_paying_costs(g, item.controller, exiled)?;
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        chapter(1, cast_from_hand, "I \u{2014} You may cast an instant or sorcery spell with mana value 1 or 2 from your hand without paying its mana cost."),
        chapter(2, effects::target_player_mills::<4>, "II \u{2014} Target player mills four cards.").targets(&[targets::PLAYER]),
        chapter(3, exile_and_copy, "III \u{2014} Exile target instant or sorcery card from your graveyard. Copy it. You may cast the copy.")
            .targets(&[targets::INSTANT_OR_SORCERY_IN_YOUR_GRAVEYARD]),
    ],
    ..oracle::FOUNDING_THE_THIRD_PATH
};
