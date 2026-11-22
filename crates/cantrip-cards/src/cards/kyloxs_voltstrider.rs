use cantrip_api::*;
use cantrip_mechanics::crew::{becomes_a_creature, crew};
use cantrip_mechanics::{choose, triggers};
use cantrip_oracle as oracle;

/// "Whenever this Vehicle attacks, you may cast an instant or sorcery spell from among cards exiled with it. If that spell would
/// be put into a graveyard, put it on the bottom of its owner's library instead." (CR 607.2a, 601.2, 614.1a, 401.4)
///
/// The cards its collect-evidence cost exiled are exiled with it (CR 607.2a, 701.59a), found as the ability resolves; a card
/// exiled with an earlier object of the same name is not (CR 400.7). The spell is cast by paying its costs: nothing is free.
/// The spell the card would be cast as is judged (CR 601.3e), so an Adventure's instant half counts. Nothing is asked when no
/// card among them can be cast.
fn cast_from_among_the_evidence(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let you = item.controller;
    let exiled = g.exiled_with(g.linked(item));
    choose::cast_up_to_among(
        g,
        you,
        item.object,
        Purpose::Cast,
        &exiled,
        1,
        CastTerms { exile_instead: ExileInstead::BottomOfLibrary, ..CastTerms::default() },
        |g, card, w| {
            let spell = g.chars_as_cast(card, w.way, you);
            spell.is(CardType::Instant) || spell.is(CardType::Sorcery)
        },
    )
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        ability(Cost { collect_evidence: 6, ..Cost::NONE }, becomes_a_creature, "Collect evidence 6: This Vehicle becomes an artifact creature until end of turn."),
        triggered(
            triggers::THIS_ATTACKS,
            cast_from_among_the_evidence,
            "Whenever this Vehicle attacks, you may cast an instant or sorcery spell from among cards exiled with it. If that spell would be put into a graveyard, put it on the bottom of its owner's library instead.",
        ),
        crew(2, "Crew 2"),
    ],
    ..oracle::KYLOXS_VOLTSTRIDER
};
