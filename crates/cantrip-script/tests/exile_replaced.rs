//! Cards that act on what "really" reached exile, when a replacement effect sends a card elsewhere instead (CR 614.6,
//! 406.2): Hex Magic draws "that many" and lets the player play "cards exiled this way", Fireglass Mentor chooses one "of
//! them", and Esper Origins puts itself onto the battlefield only once it is exiled (CR 608.2c, 712.14a). No printed card in
//! the engine replaces an exile from a hand, a library or the stack, so a test-only one does, and each card's scenario is
//! played on the card table with it (`cantrip_script::run_on`).

use std::sync::OnceLock;

use cantrip_engine::*;
use cantrip_script::{GameScript, run_on};

/// "If a card would be put into exile from a hand, a library or the stack, put it into its owner's graveyard instead."
fn exile_from_hand_library_or_stack(_: &dyn Query, _: &Asker, proposal: &Proposal) -> bool {
    matches!(proposal, Proposal::ZoneChange(z)
        if z.is_card && z.to.is(ZoneKind::Exile) && matches!(z.from.kind(), ZoneKind::Hand | ZoneKind::Library | ZoneKind::Stack))
}

fn graveyard_instead(q: &dyn Query, _: &Asker, proposal: &mut Proposal) {
    if let Proposal::ZoneChange(z) = proposal {
        z.to = Zone::graveyard(q.obj(z.object).owner);
    }
}

static REROUTER: CardDef = CardDef {
    name: "Probe Rerouter",
    oracle: true,
    has_cost: true,
    types: TypeSet::of(&[CardType::Enchantment]),
    replacements: &[replacement(ReplacementKind::ZoneChange, exile_from_hand_library_or_stack, graveyard_instead, "puts a card exiled from a hand, a library or the stack into its owner's graveyard")],
    ..CardDef::BLANK
};

/// "If a land card would be put into exile from a library, put it into its owner's graveyard instead": one of two cards
/// exiled together goes elsewhere, the other reaches exile.
fn land_exiled_from_a_library(q: &dyn Query, _: &Asker, proposal: &Proposal) -> bool {
    matches!(proposal, Proposal::ZoneChange(z)
        if z.is_card && z.to.is(ZoneKind::Exile) && z.from.is(ZoneKind::Library) && q.chars(z.object).is(CardType::Land))
}

static LAND_REROUTER: CardDef = CardDef {
    name: "Probe Land Rerouter",
    replacements: &[replacement(ReplacementKind::ZoneChange, land_exiled_from_a_library, graveyard_instead, "puts a land card exiled from a library into its owner's graveyard")],
    ..REROUTER
};

fn table() -> &'static CardTable {
    static TABLE: OnceLock<CardTable> = OnceLock::new();
    TABLE.get_or_init(|| CardTable::new(cantrip_cards::all().into_iter().chain([&REROUTER, &LAND_REROUTER])))
}

fn play(name: &str, text: &str) {
    let script = GameScript::parse(text, name).unwrap_or_else(|e| panic!("{name}: {e}"));
    if let Err(failure) = run_on(&script, table()) {
        panic!("{name}:\n{}", failure.report);
    }
}

/// The hand is put into the graveyard instead of exile: no card was exiled this way, so Hex Magic draws none and lets
/// nothing be played.
#[test]
fn hex_magic_draws_and_permits_only_for_the_cards_that_reached_exile() {
    play(
        "hex-magic-rerouted",
        "
[Bot1 Deck]
Probe Rerouter
Mox Ruby
Mox Jet
Mox Pearl
Hex Magic
Lightning Bolt
Forest
Plains x14

[Bot2 Deck]
Plains x20

[Actions]
Bot1.Hand.Probe Rerouter.Cast
Bot2.Pass
Bot1.Hand.Mox Ruby.Cast
Bot1.Hand.Mox Jet.Cast
Bot1.Hand.Mox Pearl.Cast
Bot1.Battlefield.Mox Ruby.Activate
Bot1.Battlefield.Mox Jet.Activate
Bot1.Battlefield.Mox Pearl.Activate
Bot1.Hand.Hex Magic.Cast
Bot2.Pass
? Exile.Count = 0
? Bot1.Hand.Count = 0
! Bot1.Graveyard.Forest.Play
! Bot1.Graveyard.Lightning Bolt.Cast(Bot2)
[Bot1.Main2]

[Expect]
Exile.Count = 0
Bot1.Hand.Count = 0
Bot1.Graveyard = Hex Magic; Forest; Lightning Bolt
",
    );
}

/// Of the top two cards, the Mountain goes to the graveyard instead of exile and the Lightning Bolt reaches it: the Bolt
/// is the one card "of them" to choose, so it is chosen and cast from exile, and the Mountain, never exiled, is not
/// played from the graveyard. (Were the Mountain among the choices, it would be the one chosen, as the first.)
#[test]
fn fireglass_mentor_chooses_only_among_the_cards_that_reached_exile() {
    play(
        "fireglass-mentor-rerouted",
        "
[Bot1 Deck]
Mox Jet
Mox Ruby
Black Lotus
Fireglass Mentor
Lightning Bolt
Probe Land Rerouter
Island
Mountain
Lightning Bolt
Island x12

[Bot2 Deck]
Island x20

[Actions]
Bot1.Hand.Probe Land Rerouter.Cast
Bot1.Pass
Bot2.Pass
Bot1.Hand.Mox Jet.Cast
Bot1.Pass
Bot1.Hand.Mox Ruby.Cast
Bot1.Pass
Bot1.Hand.Black Lotus.Cast
Bot1.Pass
Bot1.Battlefield.Black Lotus.Activate(Color: Red)
Bot1.Battlefield.Mox Jet.Activate
Bot1.Hand.Fireglass Mentor.Cast
Bot1.Pass
Bot1.Hand.Lightning Bolt.Cast(Bot2)
Bot1.Pass
[T1 Bot1.Main2]
Bot1.Pass
Bot2.Pass
? Exile = Lightning Bolt
! Bot1.Graveyard.Mountain.Play
Bot1.Battlefield.Mox Ruby.Activate
Bot1.Exile.Lightning Bolt.Cast(Bot2)
Bot1.Pass
Bot2.Pass

[Expect]
Bot2.Life = 14
Exile.Count = 0
Bot1.Library.Count = 12
Bot1.Graveyard = Lightning Bolt; Mountain; Lightning Bolt; Black Lotus
",
    );
}

/// Esper Origins cast from the graveyard (Underworld Breach's escape) exiles itself as it resolves, but the exile is put
/// into the graveyard instead: it never reached exile, so it is not put onto the battlefield transformed. (Turn 1 plays a
/// Forest; turn 3 draws the Rerouter and plays the rest of Esper Origins' Breach scenario.)
#[test]
fn esper_origins_enters_transformed_only_from_exile() {
    play(
        "esper-origins-rerouted",
        "
[Bot1 Deck]
Forest
Mox Emerald
Mox Ruby
Mox Jet
Black Lotus
Esper Origins
Underworld Breach
Probe Rerouter
Lightning Bolt
Island
Forest
Llanowar Elves
Forest x10

[Bot2 Deck]
Island x20

[Actions]
Bot1.Hand.Forest.Play
[T3 Bot1.Main1]
Bot1.Hand.Probe Rerouter.Cast
Bot1.Pass
Bot2.Pass
Bot1.Hand.Mox Emerald.Cast
Bot1.Hand.Mox Ruby.Cast
Bot1.Hand.Mox Jet.Cast
Bot1.Hand.Black Lotus.Cast
Bot1.Battlefield.Forest.Activate
Bot1.Battlefield.Mox Jet.Activate
Bot1.Hand.Esper Origins.Cast
Bot1.Pass
Bot1.Arrange(| Lightning Bolt; Island)
Bot1.Battlefield.Black Lotus.Activate(Color: Green)
Bot1.Battlefield.Mox Ruby.Activate
Bot1.Hand.Underworld Breach.Cast
Bot1.Pass
Bot1.Battlefield.Mox Emerald.Activate
Bot1.Graveyard.Esper Origins.Cast(Ability: Escape)
Bot1.Choose(Pay mana)
Bot1.Choose(Pay mana)
Bot1.Choose(Lightning Bolt; Island; Black Lotus)
Bot1.Pass
Bot1.Arrange(Forest; Llanowar Elves |)
? Bot1.Graveyard = Esper Origins
Bot1.Pass
Bot2.Pass

[Expect]
Bot1.Life = 24
Exile = Lightning Bolt; Island; Black Lotus
Bot1.Battlefield = Forest; Mox Emerald; Mox Ruby; Mox Jet; Underworld Breach; Probe Rerouter
Bot1.Graveyard = Esper Origins
",
    );
}
