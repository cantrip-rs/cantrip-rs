//! The line forms of prototype, surge and assist (CR 702.160, 702.117, 702.132), played on a table with test-only cards
//! (`cantrip_script::run_on`) until the printed cards that have them are added: casting prototyped, casting for a surge
//! cost and its refusal before another spell, choosing a player to assist and that player activating and paying.

use std::sync::OnceLock;

use cantrip_engine::*;
use cantrip_mechanics::surge;
use cantrip_script::{GameScript, run_on};

/// "Prototype {2}{R} — 3/2 / Haste": a 6/4 artifact creature for {7}.
static AUTOMATON: CardDef = CardDef {
    name: "Probe Automaton",
    oracle: true,
    cost: generic(7),
    types: TypeSet::of(&[CardType::Artifact, CardType::Creature]),
    subtypes: SubtypeSet::of(&[Subtype::Construct]),
    keywords: Keywords::of(&[Keyword::Haste]),
    power: Some(6),
    toughness: Some(4),
    text: "Prototype {2}{R} \u{2014} 3/2\nHaste",
    prototype: Some(Prototype { cost: mana("{2}{R}"), power: 3, toughness: 2, text: "Prototype {2}{R} \u{2014} 3/2" }),
    ..CardDef::BLANK
};

/// "Surge {1}{R}": a 3/2 for {3}{R}.
static SURGER: CardDef = CardDef {
    name: "Probe Surger",
    oracle: true,
    cost: mana("{3}{R}"),
    types: TypeSet::of(&[CardType::Creature]),
    power: Some(3),
    toughness: Some(2),
    text: "Surge {1}{R}",
    ways: &[surge::way(mana("{1}{R}"), "Surge {1}{R}")],
    ..CardDef::BLANK
};

/// "Assist": a 7/5 for {5}{G}.
static ASSISTED: CardDef = CardDef {
    name: "Probe Assisted",
    oracle: true,
    cost: mana("{5}{G}"),
    types: TypeSet::of(&[CardType::Creature]),
    power: Some(7),
    toughness: Some(5),
    text: "Assist",
    pay_with: PayWith { assist: true, ..PayWith::NONE },
    ..CardDef::BLANK
};

fn table() -> &'static CardTable {
    static TABLE: OnceLock<CardTable> = OnceLock::new();
    TABLE.get_or_init(|| CardTable::new(cantrip_cards::all().into_iter().chain([&AUTOMATON, &SURGER, &ASSISTED])))
}

fn play(name: &str, text: &str) {
    let script = GameScript::parse(text, name).unwrap_or_else(|e| panic!("{name}: {e}"));
    if let Err(failure) = run_on(&script, table()) {
        panic!("{name}:\n{}", failure.report);
    }
}

/// CR 718.3b: cast prototyped for {2}{R} of Black Lotus's {R}{R}{R}, it is a red 3/2 with mana value 3 on the battlefield.
#[test]
fn a_card_is_cast_prototyped() {
    play(
        "probe-automaton-cast-prototyped",
        "
[Bot1 Deck]
Black Lotus
Probe Automaton
Mountain x18

[Bot2 Deck]
Island x20

[Actions]
Bot1.Hand.Black Lotus.Cast
Bot1.Battlefield.Black Lotus.Activate(Color: Red)
Bot1.Hand.Probe Automaton.Cast(Ability: Prototype)
[T1 Bot1.Main2]

[Expect]
Bot1.Battlefield = Probe Automaton
Bot1.Battlefield.Probe Automaton.Prototyped = true
Bot1.Battlefield.Probe Automaton.Power = 3
Bot1.Battlefield.Probe Automaton.Toughness = 2
Bot1.Battlefield.Probe Automaton.ManaValue = 3
Bot1.Battlefield.Probe Automaton.Colors = Red
",
    );
}

/// CR 702.117a: surge is refused before Bot1 has cast another spell this turn, and cast for {1}{R} after Black Lotus.
#[test]
fn a_card_is_cast_for_its_surge_cost() {
    play(
        "probe-surger-cast-for-surge",
        "
[Bot1 Deck]
Black Lotus
Probe Surger
Mountain x18

[Bot2 Deck]
Island x20

[Actions]
! Bot1.Hand.Probe Surger.Cast(Ability: Surge)
Bot1.Hand.Black Lotus.Cast
Bot1.Battlefield.Black Lotus.Activate(Color: Red)
Bot1.Hand.Probe Surger.Cast(Ability: Surge)
[T1 Bot1.Main2]

[Expect]
Bot1.Battlefield = Probe Surger
",
    );
}

/// CR 702.132a: Bot1 casts the {5}{G} spell with five mana of their own, choosing Bot2, who taps their Forest in their window;
/// Bot1 taps their two Forests in theirs - after it nothing more may be activated (CR 601.2g) - and Bot2 pays {1} of the
/// generic mana with their {G}.
#[test]
fn another_player_assists_with_a_spells_cost() {
    play(
        "probe-assisted-with-bot2s-forest",
        "
[Bot1 Deck]
Black Lotus
Probe Assisted
Forest x18

[Bot2 Deck]
Forest x20

[Actions]
Bot1.Hand.Forest.Play
Bot2.Hand.Forest.Play
Bot1.Hand.Forest.Play
Bot1.Hand.Black Lotus.Cast
Bot1.Battlefield.Black Lotus.Activate(Color: Green)
Bot1.Hand.Probe Assisted.Cast
Bot1.Accept
Bot2.Battlefield.Forest.Activate
Bot1.Battlefield.Forest.Activate
Bot1.Battlefield.Forest.Activate
Bot2.PayCost(X: 1)
[T3 Bot1.Main2]

[Expect]
Bot1.Battlefield = Forest; Forest; Probe Assisted
Bot2.Battlefield.Forest.Tapped = true
",
    );
}
