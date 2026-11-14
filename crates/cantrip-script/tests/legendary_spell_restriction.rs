//! CR 205.4e: "A player can't cast a legendary instant or sorcery spell unless that player controls a legendary creature or a
//! legendary planeswalker." The printed legendary instants and sorceries (Urza's Ruinous Blast, Kamahl's Druidic Vow, Yawgmoth's
//! Vile Offering, ...) have no content file yet, so these scenarios are played on a table with test-only ones
//! (`cantrip_script::run_on`): a legendary sorcery and a legendary instant with nothing on them, a split card one half of which
//! is a legendary sorcery, a modal double-faced card whose back face is, a legendary sorcery of mana cost {2}, and a
//! planeswalker that is not legendary.
//!
//! The rule reads the spell as it would be cast (CR 601.3e: only the half or face cast, CR 709.3a, 712.8a) and the player's
//! permanents as they are now, through the layers (Mirrorweave's copies, Phantasmal Image's, Act of Treason's control, a
//! face-down permanent, a phased-out one), and it holds for every way of casting - from a hand, from a graveyard by flashback,
//! and from exile without paying the mana cost as cascade resolves (CR 702.85a, 608.2g) - as the cast begins and once its
//! proposal is complete (CR 601.3, 601.6), not once its costs are paid.

use std::sync::OnceLock;

use cantrip_engine::*;
use cantrip_script::{GameScript, run_on};

/// "Probe Legendary Sorcery": a legendary sorcery card with nothing on it, of mana cost {0}.
static LEGENDARY_SORCERY: CardDef = CardDef {
    name: "Probe Legendary Sorcery",
    oracle: true,
    has_cost: true,
    types: TypeSet::of(&[CardType::Sorcery]),
    supertypes: Supertypes { basic: false, legendary: true, snow: false, world: false },
    ..CardDef::BLANK
};

/// "Probe Legendary Instant": a legendary instant card with nothing on it, of mana cost {0}.
static LEGENDARY_INSTANT: CardDef = CardDef { name: "Probe Legendary Instant", types: TypeSet::of(&[CardType::Instant]), ..LEGENDARY_SORCERY };

/// A split card's halves (CR 709.3): "Probe Legend Half", a legendary sorcery, and "Probe Plain Half", a sorcery.
static LEGEND_HALF: CardDef = CardDef { name: "Probe Legend Half", ..LEGENDARY_SORCERY };
static PLAIN_HALF: CardDef = CardDef { name: "Probe Plain Half", supertypes: Supertypes { basic: false, legendary: false, snow: false, world: false }, ..LEGENDARY_SORCERY };
static SPLIT: CardDef = CardDef { name: "Probe Legend Half // Probe Plain Half", split: Some((&LEGEND_HALF, &PLAIN_HALF)), ..PLAIN_HALF };

/// A modal double-faced card (CR 712.11b): "Probe Plain Front", a sorcery, and its back face "Probe Legend Back", a legendary
/// sorcery, each with a mana cost of its own.
static LEGEND_BACK: CardDef = CardDef { name: "Probe Legend Back", ..LEGENDARY_SORCERY };
static PLAIN_FRONT: CardDef = CardDef { name: "Probe Plain Front", back: Some(&LEGEND_BACK), ..PLAIN_HALF };

/// "Probe Costly Legendary Sorcery": a legendary sorcery card with nothing on it, of mana cost {2}, so that mana abilities are
/// activated as it is paid for (CR 601.2g).
static COSTLY_LEGENDARY_SORCERY: CardDef = CardDef { name: "Probe Costly Legendary Sorcery", cost: ManaCost { generic: 2, ..ManaCost::ZERO }, ..LEGENDARY_SORCERY };

/// "Probe Plain Walker": a planeswalker card that is not legendary, of mana cost {0} and loyalty 3, with no abilities.
static PLAIN_WALKER: CardDef = CardDef {
    name: "Probe Plain Walker",
    types: TypeSet::of(&[CardType::Planeswalker]),
    supertypes: Supertypes { basic: false, legendary: false, snow: false, world: false },
    loyalty: Some(3),
    ..LEGENDARY_SORCERY
};

fn table() -> &'static CardTable {
    static TABLE: OnceLock<CardTable> = OnceLock::new();
    TABLE.get_or_init(|| {
        CardTable::new(cantrip_cards::all().into_iter().chain([
            &LEGENDARY_SORCERY,
            &LEGENDARY_INSTANT,
            &LEGEND_HALF,
            &PLAIN_HALF,
            &SPLIT,
            &LEGEND_BACK,
            &PLAIN_FRONT,
            &COSTLY_LEGENDARY_SORCERY,
            &PLAIN_WALKER,
        ]))
    })
}

fn play(name: &str, text: &str) {
    let script = GameScript::parse(text, name).unwrap_or_else(|e| panic!("{name}: {e}"));
    if let Err(failure) = run_on(&script, table()) {
        panic!("{name}:\n{}", failure.report);
    }
}

/// The reviewer's probe: five untapped Mox Pearls and no legend. Neither the legendary sorcery nor the legendary instant is
/// offered, with mana or without (the refusal is judged again with mana to spare), and both stay in hand.
#[test]
fn a_legendary_sorcery_or_instant_is_not_cast_without_a_legend() {
    play(
        "legendary-spells-are-refused-without-a-legend",
        "
[Bot1 Deck]
Mox Pearl x5
Probe Legendary Sorcery
Probe Legendary Instant
Island x20

[Bot2 Deck]
Island x20

[Actions]
Bot1.Hand.Mox Pearl.Cast
Bot1.Hand.Mox Pearl.Cast
Bot1.Hand.Mox Pearl.Cast
Bot1.Hand.Mox Pearl.Cast
Bot1.Hand.Mox Pearl.Cast
Bot1.Pass
Bot2.Pass
! Bot1.Hand.Probe Legendary Sorcery.Cast
! Bot1.Hand.Probe Legendary Instant.Cast
[T1 Bot1.Main2]

[Expect]
Turn = 1
Bot1.Battlefield.Count = 5
Bot1.Hand = Probe Legendary Sorcery; Probe Legendary Instant
Bot1.Graveyard.Count = 0
",
    );
}

/// With Isamaru, Hound of Konda, a legendary creature, on the battlefield, both are cast and resolve.
#[test]
fn a_legendary_creature_lets_its_controller_cast_legendary_spells() {
    play(
        "legendary-spells-are-cast-with-a-legendary-creature",
        "
[Bot1 Deck]
Mox Pearl
Isamaru, Hound of Konda
Probe Legendary Sorcery
Probe Legendary Instant
Island x20

[Bot2 Deck]
Island x20

[Actions]
Bot1.Hand.Mox Pearl.Cast
Bot1.Battlefield.Mox Pearl.Activate
Bot1.Hand.Isamaru, Hound of Konda.Cast
Bot1.Hand.Probe Legendary Sorcery.Cast
Bot1.Hand.Probe Legendary Instant.Cast
[T1 Bot1.Main2]

[Expect]
Turn = 1
Bot1.Battlefield = Isamaru, Hound of Konda; Mox Pearl
Bot1.Graveyard = Probe Legendary Sorcery; Probe Legendary Instant
",
    );
}

/// With Jace, the Mind Sculptor, a legendary planeswalker, on the battlefield, the legendary sorcery is cast.
#[test]
fn a_legendary_planeswalker_lets_its_controller_cast_legendary_spells() {
    play(
        "legendary-sorcery-is-cast-with-a-legendary-planeswalker",
        "
[Bot1 Deck]
Black Lotus
Mox Sapphire
Jace, the Mind Sculptor
Probe Legendary Sorcery
Island x20

[Bot2 Deck]
Island x20

[Actions]
Bot1.Hand.Black Lotus.Cast
Bot1.Hand.Mox Sapphire.Cast
Bot1.Battlefield.Black Lotus.Activate(Color: Blue)
Bot1.Battlefield.Mox Sapphire.Activate
Bot1.Hand.Jace, the Mind Sculptor.Cast
Bot1.Hand.Probe Legendary Sorcery.Cast
[T1 Bot1.Main2]

[Expect]
Turn = 1
Bot1.Battlefield = Jace, the Mind Sculptor; Mox Sapphire
Bot1.Graveyard = Probe Legendary Sorcery; Black Lotus
",
    );
}

/// A legendary land (Gaea's Cradle) and a legendary artifact (Mox Amber) are legends, but neither a creature nor a
/// planeswalker: the legendary sorcery is still refused.
#[test]
fn a_legendary_land_or_artifact_is_not_enough() {
    play(
        "legendary-sorcery-is-refused-with-only-a-legendary-land-and-artifact",
        "
[Bot1 Deck]
Gaea's Cradle
Mox Amber
Probe Legendary Sorcery
Island x20

[Bot2 Deck]
Island x20

[Actions]
Bot1.Hand.Gaea's Cradle.Play
Bot1.Hand.Mox Amber.Cast
Bot1.Pass
Bot2.Pass
! Bot1.Hand.Probe Legendary Sorcery.Cast
[T1 Bot1.Main2]

[Expect]
Turn = 1
Bot1.Battlefield = Gaea's Cradle; Mox Amber
Bot1.Hand = Island; Island; Island; Island; Probe Legendary Sorcery
",
    );
}

/// The legend must be the caster's own: Bot2's Isamaru lets Bot1 cast nothing, at Bot2's end step. On Bot1's turn Phantasmal
/// Image enters as a copy of it, a legendary creature Bot1 controls (CR 707.2), and the legendary sorcery is cast.
#[test]
fn an_opponents_legend_is_not_enough_and_a_copy_of_it_is() {
    play(
        "legendary-spells-need-a-legend-of-the-casters-own",
        "
[Bot1 Deck]
Black Lotus
Phantasmal Image
Probe Legendary Sorcery
Probe Legendary Instant
Island x20

[Bot2 Deck]
Mox Pearl
Isamaru, Hound of Konda
Island x20

[Actions]
[T2 Bot2.Main1]
Bot2.Hand.Mox Pearl.Cast
Bot2.Battlefield.Mox Pearl.Activate
Bot2.Hand.Isamaru, Hound of Konda.Cast
[T2 Bot2.End]
! Bot1.Hand.Probe Legendary Instant.Cast
[T3 Bot1.Main1]
! Bot1.Hand.Probe Legendary Sorcery.Cast
Bot1.Hand.Black Lotus.Cast
Bot1.Battlefield.Black Lotus.Activate(Color: Blue)
Bot1.Hand.Phantasmal Image.Cast
Bot1.Accept
Bot1.Pass
Bot2.Pass
Bot1.Hand.Probe Legendary Sorcery.Cast
[T3 Bot1.Main2]

[Expect]
Turn = 3
Bot1.Battlefield = Isamaru, Hound of Konda
Bot2.Battlefield = Isamaru, Hound of Konda; Mox Pearl
Bot1.Graveyard = Probe Legendary Sorcery; Black Lotus
",
    );
}

/// The legend is judged now, through the layers: on Bot1's third turn Isamaru lets Bot1 cast the legendary instant it drew,
/// then Mirrorweave makes Isamaru a copy of Ornithopter until end of turn - no longer legendary (CR 707.2) - and the
/// legendary sorcery is refused.
#[test]
fn a_legend_that_is_no_longer_legendary_is_not_enough() {
    play(
        "legendary-sorcery-is-refused-once-mirrorweave-copies-the-legend",
        "
[Bot1 Deck]
Mox Pearl
Black Lotus
Mox Sapphire
Isamaru, Hound of Konda
Ornithopter
Mirrorweave
Probe Legendary Sorcery
Probe Legendary Instant
Island x20

[Bot2 Deck]
Island x20

[Actions]
Bot1.Hand.Mox Pearl.Cast
Bot1.Battlefield.Mox Pearl.Activate
Bot1.Hand.Isamaru, Hound of Konda.Cast
Bot1.Hand.Ornithopter.Cast
Bot1.Hand.Black Lotus.Cast
Bot1.Hand.Mox Sapphire.Cast
[T3 Bot1.Main1]
Bot1.Hand.Probe Legendary Instant.Cast
Bot1.Battlefield.Black Lotus.Activate(Color: White)
Bot1.Battlefield.Mox Sapphire.Activate
Bot1.Hand.Mirrorweave.Cast(Ornithopter)
[T3 Bot1.Main2]
! Bot1.Hand.Probe Legendary Sorcery.Cast

[Expect]
Turn = 3
Bot1.Battlefield = Mox Pearl; Mox Sapphire; Ornithopter; Ornithopter
Bot1.Graveyard = Probe Legendary Instant; Mirrorweave; Black Lotus
Bot1.Hand = Probe Legendary Sorcery
",
    );
}

/// Cascade's "you may cast it without paying its mana cost" (CR 702.85a), a cast from exile as an ability resolves: with no
/// legend, the legendary sorcery it exiles may not be cast, so Bot1 is not asked, and it goes to the bottom of the library
/// with the rest. With Isamaru on the battlefield, it is cast.
#[test]
fn cascade_casts_a_legendary_sorcery_only_for_a_player_with_a_legend() {
    play(
        "cascade-does-not-cast-a-legendary-sorcery-without-a-legend",
        "
[Bot1 Deck]
Black Lotus
Mox Emerald
Bloodbraid Elf
Island x4
Probe Legendary Sorcery
Island x10

[Bot2 Deck]
Island x24

[Actions]
Bot1.Hand.Black Lotus.Cast
Bot1.Hand.Mox Emerald.Cast
Bot1.Battlefield.Black Lotus.Activate(Color: Red)
Bot1.Battlefield.Mox Emerald.Activate
Bot1.Hand.Bloodbraid Elf.Cast
Bot1.Pass
! Bot1.Accept
Bot2.Pass
[T1 Bot1.Main2]

[Expect]
Turn = 1
Bot1.Battlefield = Bloodbraid Elf; Mox Emerald
Bot1.Graveyard = Black Lotus
Bot1.Library.Count = 11
",
    );
    play(
        "cascade-casts-a-legendary-sorcery-with-a-legend",
        "
[Bot1 Deck]
Black Lotus
Mox Emerald
Bloodbraid Elf
Mox Pearl
Isamaru, Hound of Konda
Island x2
Probe Legendary Sorcery
Island x10

[Bot2 Deck]
Island x24

[Actions]
Bot1.Hand.Mox Pearl.Cast
Bot1.Battlefield.Mox Pearl.Activate
Bot1.Hand.Isamaru, Hound of Konda.Cast
Bot1.Hand.Black Lotus.Cast
Bot1.Hand.Mox Emerald.Cast
Bot1.Battlefield.Black Lotus.Activate(Color: Red)
Bot1.Battlefield.Mox Emerald.Activate
Bot1.Hand.Bloodbraid Elf.Cast
Bot1.Accept
Bot1.Pass
Bot2.Pass
[T1 Bot1.Main2]

[Expect]
Turn = 1
Bot1.Battlefield = Bloodbraid Elf; Isamaru, Hound of Konda; Mox Emerald; Mox Pearl
Bot1.Graveyard = Probe Legendary Sorcery; Black Lotus
Bot1.Library.Count = 10
",
    );
}

/// Only the half or face cast is judged (CR 709.3a, 712.8a, 601.3e): with no legend, a split card's legendary half and a
/// modal double-faced card's legendary back face are refused, and the nonlegendary other half and front face are cast.
#[test]
fn only_the_half_or_face_cast_is_judged() {
    play(
        "legendary-half-and-back-face-are-refused-and-the-others-cast",
        "
[Bot1 Deck]
Probe Legend Half // Probe Plain Half
Probe Plain Front
Island x20

[Bot2 Deck]
Island x20

[Actions]
[T1 Bot1.Main1]
! Bot1.Hand.Probe Legend Half // Probe Plain Half.Cast(Ability: Probe Legend Half)
! Bot1.Hand.Probe Plain Front.Cast(Ability: Probe Legend Back)
Bot1.Hand.Probe Legend Half // Probe Plain Half.Cast(Ability: Probe Plain Half)
Bot1.Hand.Probe Plain Front.Cast
[T1 Bot1.Main2]

[Expect]
Turn = 1
Bot1.Graveyard = Probe Plain Front; Probe Legend Half // Probe Plain Half
",
    );
}

/// With a static ability that functions outside the battlefield (Arcane Adaptation's "creature spells you control"), each
/// cast is judged on the spell put on the stack for the moment (`ops::as_cast`) rather than on the card where it is: the
/// legendary sorcery is refused there too without a legend, and cast once Isamaru is on the battlefield.
#[test]
fn a_spell_judged_on_the_stack_is_refused_and_allowed_alike() {
    play(
        "legendary-sorcery-judged-on-the-stack-under-arcane-adaptation",
        "
[Bot1 Deck]
Black Lotus
Arcane Adaptation
Mox Pearl
Isamaru, Hound of Konda
Probe Legendary Sorcery
Island x20

[Bot2 Deck]
Island x20

[Actions]
Bot1.Hand.Black Lotus.Cast
Bot1.Battlefield.Black Lotus.Activate(Color: Blue)
Bot1.Hand.Arcane Adaptation.Cast
Bot1.Pass
Bot2.Pass
! Bot1.Hand.Probe Legendary Sorcery.Cast
Bot1.Hand.Mox Pearl.Cast
Bot1.Battlefield.Mox Pearl.Activate
Bot1.Hand.Isamaru, Hound of Konda.Cast
Bot1.Hand.Probe Legendary Sorcery.Cast
[T1 Bot1.Main2]

[Expect]
Turn = 1
Bot1.Battlefield = Arcane Adaptation; Isamaru, Hound of Konda; Mox Pearl
Bot1.Graveyard = Probe Legendary Sorcery; Black Lotus
",
    );
}

/// A cast from a graveyard by a permission (Snapcaster Mage's flashback, CR 702.34a) is a cast like any (CR 601.3): Bot2's
/// Duress takes the legendary sorcery from Bot1's hand, and on Bot1's next turn Snapcaster Mage gives it flashback. With no
/// legend the flashback cast is refused; once Isamaru is on the battlefield it is cast, and exiled as it resolves.
#[test]
fn a_flashback_cast_from_a_graveyard_needs_a_legend_too() {
    play(
        "legendary-sorcery-flashback-is-refused-without-a-legend-and-cast-with-one",
        "
[Bot1 Deck]
Snapcaster Mage
Probe Legendary Sorcery
Mox Pearl
Isamaru, Hound of Konda
Island x3
Island x10

[Bot2 Deck]
Swamp
Duress
Swamp x18

[Actions]
Bot1.Hand.Island.Play
Bot1.Hand.Mox Pearl.Cast
[T2 Bot2.Main1]
Bot2.Hand.Swamp.Play
Bot2.Battlefield.Swamp.Activate
Bot2.Hand.Duress.Cast(Bot1)
Bot2.Choose(Probe Legendary Sorcery)
[T3 Bot1.Main1]
Bot1.Hand.Island.Play
Bot1.Battlefield.Island#1.Activate
Bot1.Battlefield.Island#2.Activate
Bot1.Hand.Snapcaster Mage.Cast
Bot1.Pass
Bot2.Pass
Bot1.Pass
Bot2.Pass
? Stack.Count = 0
! Bot1.Graveyard.Probe Legendary Sorcery.Cast
Bot1.Battlefield.Mox Pearl.Activate
Bot1.Hand.Isamaru, Hound of Konda.Cast
Bot1.Pass
Bot2.Pass
Bot1.Graveyard.Probe Legendary Sorcery.Cast
[T3 Bot1.Main2]

[Expect]
Turn = 3
Exile = Probe Legendary Sorcery
Bot1.Battlefield = Island; Island; Mox Pearl; Snapcaster Mage; Isamaru, Hound of Konda
",
    );
}

/// The restriction is judged as the cast begins and once its proposal is complete (CR 601.2, 601.3, 601.6), never after: a
/// legend sacrificed for mana as the spell is paid for (Ashnod's Altar, CR 601.2g) leaves the cast as it is (the rulings of
/// Urza's Ruinous Blast and Kamahl's Druidic Vow: "Once you begin to cast a legendary sorcery, losing control of your
/// legendary creatures and planeswalkers won't affect that spell").
#[test]
fn a_legend_sacrificed_to_pay_for_the_spell_does_not_take_the_cast_back() {
    play(
        "legendary-sorcery-paid-by-sacrificing-the-only-legend-is-cast",
        "
[Bot1 Deck]
Black Lotus
Ashnod's Altar
Mox Pearl
Isamaru, Hound of Konda
Probe Costly Legendary Sorcery
Island x2
Island x10

[Bot2 Deck]
Island x20

[Actions]
Bot1.Hand.Black Lotus.Cast
Bot1.Battlefield.Black Lotus.Activate(Color: White)
Bot1.Hand.Ashnod's Altar.Cast
Bot1.Hand.Mox Pearl.Cast
Bot1.Battlefield.Mox Pearl.Activate
Bot1.Hand.Isamaru, Hound of Konda.Cast
Bot1.Pass
Bot2.Pass
Bot1.Hand.Probe Costly Legendary Sorcery.Cast
Bot1.Battlefield.Ashnod's Altar.Activate(Sacrifice: Isamaru, Hound of Konda)
Bot1.Pass
Bot2.Pass
[T1 Bot1.Main2]

[Expect]
Turn = 1
Stack.Count = 0
Bot1.Mana =
Bot1.Battlefield = Ashnod's Altar; Mox Pearl
Bot1.Graveyard = Probe Costly Legendary Sorcery; Isamaru, Hound of Konda; Black Lotus
",
    );
}

/// "Controls" is control now (CR 110.2, 613.1b): Bot1 gains control of Bot2's Isamaru with Act of Treason and casts the
/// legendary sorcery, refused a moment before.
#[test]
fn a_legend_gained_control_of_is_enough() {
    play(
        "legendary-sorcery-is-cast-with-a-legend-gained-by-act-of-treason",
        "
[Bot1 Deck]
Black Lotus
Act of Treason
Probe Legendary Sorcery
Island x4
Island x10

[Bot2 Deck]
Mox Pearl
Isamaru, Hound of Konda
Island x20

[Actions]
[T2 Bot2.Main1]
Bot2.Hand.Mox Pearl.Cast
Bot2.Battlefield.Mox Pearl.Activate
Bot2.Hand.Isamaru, Hound of Konda.Cast
[T3 Bot1.Main1]
! Bot1.Hand.Probe Legendary Sorcery.Cast
Bot1.Hand.Black Lotus.Cast
Bot1.Battlefield.Black Lotus.Activate(Color: Red)
Bot1.Hand.Act of Treason.Cast(Isamaru, Hound of Konda)
Bot1.Pass
Bot2.Pass
? Bot1.Battlefield = Isamaru, Hound of Konda
Bot1.Hand.Probe Legendary Sorcery.Cast
Bot1.Pass
Bot2.Pass
[T3 Bot1.Main2]

[Expect]
Turn = 3
Stack.Count = 0
Bot1.Battlefield = Isamaru, Hound of Konda
Bot1.Graveyard = Probe Legendary Sorcery; Act of Treason; Black Lotus
",
    );
}

/// A phased-out legend is treated as though it does not exist (CR 702.26b): Bot1 casts the legendary instant with Isamaru on
/// the battlefield, then Teferi's Protection phases out every permanent Bot1 controls, and the legendary sorcery is refused.
#[test]
fn a_phased_out_legend_is_not_enough() {
    play(
        "legendary-sorcery-is-refused-once-teferis-protection-phases-out-the-legend",
        "
[Bot1 Deck]
Mox Pearl
Black Lotus
Isamaru, Hound of Konda
Teferi's Protection
Probe Legendary Sorcery
Probe Legendary Instant
Island
Island x10

[Bot2 Deck]
Island x20

[Actions]
Bot1.Hand.Mox Pearl.Cast
Bot1.Battlefield.Mox Pearl.Activate
Bot1.Hand.Isamaru, Hound of Konda.Cast
Bot1.Pass
Bot2.Pass
Bot1.Hand.Probe Legendary Instant.Cast
Bot1.Pass
Bot2.Pass
Bot1.Hand.Black Lotus.Cast
Bot1.Battlefield.Black Lotus.Activate(Color: White)
Bot1.Hand.Teferi's Protection.Cast
Bot1.Pass
Bot2.Pass
? Bot1.PhasedOut = Isamaru, Hound of Konda; Mox Pearl
! Bot1.Hand.Probe Legendary Sorcery.Cast
[T1 Bot1.Main2]

[Expect]
Turn = 1
Bot1.Hand = Island; Probe Legendary Sorcery
Bot1.Graveyard = Black Lotus; Probe Legendary Instant
",
    );
}

/// A face-down creature has no supertypes (CR 708.2): Cryptic Coat cloaks Isamaru, the top card of Bot1's library, and the
/// legendary sorcery is refused; turned face up for its mana cost (CR 701.58a), Isamaru is a legend again and the sorcery is
/// cast. A planeswalker that is not legendary, and a creature that is not, are not enough either.
#[test]
fn a_face_down_legend_or_a_nonlegendary_planeswalker_is_not_enough() {
    play(
        "legendary-sorcery-is-refused-with-a-face-down-legend-and-cast-once-it-is-face-up",
        "
[Bot1 Deck]
Black Lotus
Cryptic Coat
Mox Pearl
Probe Legendary Sorcery
Probe Plain Walker
Ornithopter
Island
Isamaru, Hound of Konda
Island x10

[Bot2 Deck]
Island x20

[Actions]
Bot1.Hand.Probe Plain Walker.Cast
Bot1.Hand.Ornithopter.Cast
Bot1.Pass
Bot2.Pass
? Bot1.Battlefield = Ornithopter; Probe Plain Walker
! Bot1.Hand.Probe Legendary Sorcery.Cast
Bot1.Hand.Black Lotus.Cast
Bot1.Battlefield.Black Lotus.Activate(Color: Blue)
Bot1.Hand.Cryptic Coat.Cast
Bot1.Pass
Bot2.Pass
Bot1.Pass
Bot2.Pass
? Bot1.Battlefield..FaceDownName = Isamaru, Hound of Konda
! Bot1.Hand.Probe Legendary Sorcery.Cast
Bot1.Hand.Mox Pearl.Cast
Bot1.Battlefield.Mox Pearl.Activate
Bot1.Battlefield.#1.Activate
? Bot1.Battlefield.Isamaru, Hound of Konda.FaceDown = (absent)
Bot1.Hand.Probe Legendary Sorcery.Cast
Bot1.Pass
Bot2.Pass
[T1 Bot1.Main2]

[Expect]
Turn = 1
Stack.Count = 0
Bot1.Graveyard = Probe Legendary Sorcery; Black Lotus
",
    );
}
