use cantrip_api::*;
use cantrip_mechanics::{effects, targets, triggers};
use cantrip_oracle as oracle;

/// "Target instant or sorcery card from an opponent's graveyard" (CR 115.1, 404.1).
fn instant_or_sorcery_in_an_opponents_graveyard(q: &dyn Query, asker: &Asker, t: Target) -> bool {
    targets::card_in_an_opponents_graveyard(q, asker, t)
        && matches!(t, Target::Object(id) if q.chars(id).is(CardType::Instant) || q.chars(id).is(CardType::Sorcery))
}

/// "Exile target instant or sorcery card from an opponent's graveyard. You may cast it this turn, and mana of any type can be
/// spent to cast that spell. If that spell would be put into a graveyard, exile it instead." The permission is for the card
/// exiled, a new object (CR 400.7), for its own costs and timing; the replacement follows the spell it makes (CR 614.1a), and
/// applies only to its move to the graveyard. Nothing happens once the target is gone (CR 608.2b); a card the exile did not move
/// (a replacement sent it elsewhere) is not castable from where it went.
fn exile_and_may_cast(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(card) = targets::object(g, item, 0) else { return Ok(()) };
    let exiled = g.exile(card);
    g.end_sentence();
    g.allow_play(exiled, item.controller, PlayGrant { expiry: Expiry::EndOfTurn, any_type: true, exile_instead: ExileInstead::Spell, ..PlayGrant::CAST });
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            triggers::YOU_CAST_A_SPELL_YOU_DONT_OWN,
            effects::plus_one_counter_on_each_creature_you_control,
            "Whenever you cast a spell you don't own, put a +1/+1 counter on each creature you control.",
        ),
        ability(
            cost("{2}").and(sacrifice(1, targets::filters::another_creature)),
            exile_and_may_cast,
            "{2}, Sacrifice another creature: Exile target instant or sorcery card from an opponent's graveyard. You may cast it this turn, and mana of any type can be spent to cast that spell. If that spell would be put into a graveyard, exile it instead. Activate only as a sorcery.",
        )
        .sorcery_speed()
        .targets(&[target(instant_or_sorcery_in_an_opponents_graveyard)]),
    ],
    ..oracle::NITA_FORUM_CONCILIATOR
};
