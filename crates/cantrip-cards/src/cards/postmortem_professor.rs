use cantrip_api::*;
use cantrip_mechanics::{effects, triggers};
use cantrip_oracle as oracle;

/// "An instant or sorcery card" in the graveyard, the card the ability's cost exiles.
fn instant_or_sorcery(q: &dyn Query, _: &Asker, id: ObjectId) -> bool {
    let chars = q.chars(id);
    chars.is(CardType::Instant) || chars.is(CardType::Sorcery)
}

pub static CARD: CardDef = CardDef {
    keywords: Keywords::of(&[Keyword::CantBlock]),
    abilities: &[
        triggered(
            triggers::THIS_ATTACKS,
            effects::each_opponent_loses_1_life_you_gain_1,
            "Whenever this creature attacks, each opponent loses 1 life and you gain 1 life.",
        ),
        // It returns this card from the graveyard, so it functions only there (CR 113.6m); the card exiled as its cost is
        // one of the controller's own and never this card, a creature.
        ability(
            cost("{1}{B}").and(exile_from_graveyard_of(1, instant_or_sorcery)),
            effects::return_this_card_to_battlefield,
            "{1}{B}, Exile an instant or sorcery card from your graveyard: Return this card from your graveyard to the battlefield.",
        )
        .from(ZoneKind::Graveyard),
    ],
    ..oracle::POSTMORTEM_PROFESSOR
};
