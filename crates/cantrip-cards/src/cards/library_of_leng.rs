use cantrip_api::*;
use cantrip_oracle as oracle;

/// "If an effect causes you to discard a card ... instead of into your graveyard" (CR 614.1a, 701.9a): a card of this
/// Library's controller, discarded from their hand to their graveyard by a spell's or ability's effect, whoever controls it
/// (CR 609.1; the rulings). A discard that pays a cost the player is paying as they cast or activate, and the cleanup step's
/// (CR 514.1, a rule's), have no effect causing them (`Query::effect_controller`), so the card goes to the graveyard (the
/// rulings: "costs aren't effects"). A discard a resolving spell or ability has a player make is its effect's, an "unless"
/// payment's included - CR 701.9c's illegal cost payment is one such discard put into a hidden zone - but a card discarded
/// to pay a cost that specifies a characteristic of it is never offered this (CR 701.9c; the owner's ruling, the core's
/// `ZoneChange::pays_characteristic`). The discard is still a discard (CR 701.9a): what triggers on one sees it, and a card
/// put into the library unrevealed has its characteristics undefined (CR 701.9c).
fn an_effect_discards_your_card(q: &dyn Query, asker: &Asker, proposal: &Proposal) -> bool {
    matches!(proposal, Proposal::ZoneChange(z)
        if z.reason == ZoneReason::Discard
            && q.obj(z.object).owner == asker.controller
            && z.to == Zone::graveyard(asker.controller)
            && q.effect_controller().is_some())
}

/// "... you may put it on top of your library": the card is not revealed as it goes unless the discarding effect revealed
/// it (the rulings). Several put there by one discard are in the order their owner arranges (CR 401.4), which the core asks.
fn on_top_of_your_library(_: &dyn Query, asker: &Asker, proposal: &mut Proposal) {
    if let Proposal::ZoneChange(z) = proposal {
        z.to = Zone::library(asker.controller);
    }
}

const ON_TOP: Question = Question::new(
    "PutOnTopOfLibrary",
    "put the discarded card on top of your library instead of into your graveyard?",
    "",
    "Top of library",
    "Graveyard",
);

pub static CARD: CardDef = CardDef {
    // "You have no maximum hand size" (CR 402.2, 514.1).
    play_rules: &[PlayRule::MaximumHandSize(None)],
    replacements: &[replacement(
        ReplacementKind::ZoneChange,
        an_effect_discards_your_card,
        on_top_of_your_library,
        "may put a card you discard on top of your library",
    )
    .optional(&ON_TOP)],
    ..oracle::LIBRARY_OF_LENG
};
