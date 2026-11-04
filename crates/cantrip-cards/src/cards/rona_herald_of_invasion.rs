use cantrip_api::*;
use cantrip_mechanics::{effects, library, triggers};
use cantrip_oracle as oracle;

const PUT_ONTO_THE_BATTLEFIELD: Question = Question::new(
    "PutExiledLandOntoBattlefield",
    "put the exiled land card onto the battlefield under your control?",
    "It was exiled from the hand of the controller of the source that dealt damage to Rona",
    "Put it onto the battlefield",
    "Leave it exiled",
);

/// "Whenever a source deals damage to Rona, that source's controller exiles a card from their hand at random. If it's a land card,
/// you may put it onto the battlefield under your control. Otherwise, you may cast it without paying its mana cost." (CR 608.2c,
/// 406.2, 305.1). The source's controller is the one it had when it dealt the damage (CR 608.2h); the card is exiled whatever it is,
/// and "it" is that card in exile.
fn sources_controller_exiles_card_their(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(source) = item.subject.object() else { return Ok(()) };
    let hand = g.state().zone(Zone::hand(g.controller(source))).to_vec();
    let Some(card) = library::random_of(g, &hand) else { return Ok(()) };
    let Some(&exiled) = g.exile_all(&[card]).first() else { return Ok(()) };
    g.end_sentence();
    if g.chars(exiled).is(CardType::Land) {
        if effects::ask_may(g, item.controller, item.object, Purpose::Ask(&PUT_ONTO_THE_BATTLEFIELD))? {
            g.put_onto_battlefield(exiled, item.controller, false)?;
        }
    } else {
        effects::may_cast_without_paying(g, item.controller, exiled)?;
    }
    Ok(())
}

pub static BACK: CardDef = CardDef {
    abilities: &[
        triggered(
            triggers::A_SOURCE_DEALS_DAMAGE_TO_THIS,
            sources_controller_exiles_card_their,
            "Whenever a source deals damage to Rona, that source's controller exiles a card from their hand at random. If it's a land card, you may put it onto the battlefield under your control. Otherwise, you may cast it without paying its mana cost.",
        ),
    ],
    ..oracle::RONA_TOLARIAN_OBLITERATOR
};

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(triggers::LEGENDARY_SPELL_YOU_CAST, effects::untap_self, "Whenever you cast a legendary spell, untap Rona."),
        ability(TAP, effects::loot, "{T}: Draw a card, then discard a card."),
        ability(cost("{5}{B/P}"), effects::transform_self, "{5}{B/P}: Transform Rona. Activate only as a sorcery.").sorcery_speed(),
    ],
    back: Some(&BACK),
    ..oracle::RONA_HERALD_OF_INVASION
};
