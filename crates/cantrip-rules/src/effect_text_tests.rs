//! Resolved prevention and replacement effects keep the text which created them
//! (CR 611.2c, 612.1), while the damage source's current qualities remain live.
use crate::{Cx, Game, Setup};
use cantrip_api::*;

const YOU: PlayerId = PlayerId(0);
thread_local! {
    static PRINTED_WORD: std::cell::Cell<Option<Color>> = const { std::cell::Cell::new(None) };
}

fn observe_live_text(q: &dyn Query, asker: &Asker, _: &Proposal) -> bool {
    PRINTED_WORD.with(|seen| seen.set(Some(q.words(asker.source).color(Color::Red))));
    false
}

fn red_word(q: &dyn Query, asker: &Asker, source: ObjectId) -> bool {
    q.chars(source).colors.has(q.words(asker.source).color(Color::Red))
}

fn prevents(q: &dyn Query, asker: &Asker, _: ObjectId, d: &DamageEvent) -> bool {
    red_word(q, asker, d.source)
}

fn replaces(q: &dyn Query, asker: &Asker, _: ObjectId, p: &Proposal) -> bool {
    matches!(p, Proposal::Damage(d) if red_word(q, asker, d.source))
}

fn replace(q: &dyn Query, asker: &Asker, _: ObjectId, p: &mut Proposal) {
    if let Proposal::Damage(d) = p {
        d.amount = if q.words(asker.source).color(Color::Red) == Color::Blue { 1 } else { 2 };
    }
}

fn after(g: &mut dyn cantrip_api::Game, asker: &Asker, _: ObjectId, _: &Proposal, _: &Proposal) {
    if g.words(asker.source).color(Color::Red) == Color::Blue {
        g.gain_life(YOU, 4);
    }
}

static SOURCE: CardDef = CardDef {
    name: "Effect Text Source",
    types: TypeSet::of(&[CardType::Artifact]),
    granted_preventions: &[GrantedPreventionDef {
        hint: "prevent red damage", damage: "red damage", of_it: None,
        applies: prevents, then: None, text: "Prevent all red damage.",
    }],
    granted_replacements: &[
        granted_replacement(ReplacementKind::Damage, replaces, replace, "replace red damage").then(after),
    ],
    replacements: &[replacement(ReplacementKind::LifeChange, observe_live_text, |_, _, _| {}, "observe current text")],
    ..CardDef::BLANK
};
static RED: CardDef = CardDef {
    name: "Effect Text Red Source", colors: Some(ColorSet::of(Color::Red)),
    types: TypeSet::of(&[CardType::Artifact]), ..CardDef::BLANK
};
static BLUE: CardDef = CardDef {
    name: "Effect Text Blue Source", colors: Some(ColorSet::of(Color::Blue)), ..RED
};

fn table() -> (Game, ObjectId, ObjectId, ObjectId) {
    let cards = Box::leak(Box::new(CardTable::new([&SOURCE, &RED, &BLUE])));
    let mut g = Game::new(cards, &Setup { decks: vec![vec![], vec![]], ..Default::default() });
    g.state.update_turn(|t| t.number = 1);
    let mut put = |card| {
        let id = g.state.push_obj(Obj::new(cards.id_of(card), YOU, Zone::BATTLEFIELD));
        g.state.zone_push(Zone::BATTLEFIELD, id); id
    };
    let source = put(&SOURCE); let red = put(&RED); let blue = put(&BLUE);
    (g, source, red, blue)
}

fn cx(g: &mut Game) -> Cx<'_> {
    Cx { g, frame: usize::MAX, cursor: 0, mana_source: None, entering: None,
        lineage: vec![], damage: None, asked_at: None, doer: None, producing: None }
}

fn change(cx: &mut Cx, source: ObjectId, from: Color, to: Color) {
    cantrip_api::Game::add_effect(cx, source, source,
        EffectKind::ChangeText { from: Word::Color(from), to: Word::Color(to) }, Duration::Indefinite);
    cantrip_api::Game::end_sentence(cx);
}

fn shield(cx: &mut Cx, source: ObjectId) {
    let card = cx.s().obj(source).card;
    cantrip_api::Game::add_effect(cx, source, source,
        EffectKind::Prevention { card, index: 0, to: Target::Player(YOU), shield: Shield::All }, Duration::EndOfTurn);
    cantrip_api::Game::end_sentence(cx);
}

#[test]
fn a_prevention_does_not_follow_a_later_text_change_on_its_source() {
    let (mut g, source, red, blue) = table(); let mut cx = cx(&mut g);
    shield(&mut cx, source); change(&mut cx, source, Color::Red, Color::Blue);
    cantrip_api::Game::damage(&mut cx, red, Target::Player(YOU), 3);
    assert_eq!(cx.s().player(YOU).life, 20, "the resolved effect still prevents red");
    cantrip_api::Game::damage(&mut cx, blue, Target::Player(YOU), 3);
    assert_eq!(cx.s().player(YOU).life, 17, "it did not become prevention of blue");
}

#[test]
fn a_prevention_keeps_a_nonempty_text_change_present_when_it_was_created() {
    let (mut g, source, red, blue) = table(); let mut cx = cx(&mut g);
    change(&mut cx, source, Color::Red, Color::Blue); shield(&mut cx, source);
    change(&mut cx, source, Color::Blue, Color::Green);
    cantrip_api::Game::damage(&mut cx, blue, Target::Player(YOU), 3);
    assert_eq!(cx.s().player(YOU).life, 20, "the effect was created to prevent blue");
    cantrip_api::Game::damage(&mut cx, red, Target::Player(YOU), 3);
    assert_eq!(cx.s().player(YOU).life, 17);
}

#[test]
fn a_player_replacement_uses_its_creation_text_in_matching_applying_and_then() {
    let (mut g, source, _, blue) = table(); let mut cx = cx(&mut g);
    change(&mut cx, source, Color::Red, Color::Blue);
    let card = cx.s().obj(source).card;
    cantrip_api::Game::add_player_effect(&mut cx, YOU, source,
        EffectKind::ReplacementRule { card, index: 0 }, Duration::EndOfTurn);
    cantrip_api::Game::end_sentence(&mut cx);
    change(&mut cx, source, Color::Blue, Color::Green);
    cantrip_api::Game::damage(&mut cx, blue, Target::Player(YOU), 3);
    assert_eq!(cx.s().player(YOU).life, 23, "replace with one damage, then gain four");
    assert_eq!(PRINTED_WORD.with(|seen| seen.get()), Some(Color::Green), "the printed replacement asked during the follow-up reads current text");
    assert_eq!(Query::words(&cx, source).color(Color::Red), Color::Green, "callback scope ended");
}

#[test]
fn an_object_replacement_also_retains_its_creation_text() {
    let (mut g, source, _, blue) = table(); let mut cx = cx(&mut g);
    change(&mut cx, source, Color::Red, Color::Blue);
    let card = cx.s().obj(source).card;
    cantrip_api::Game::add_effect(&mut cx, source, source,
        EffectKind::Replacement { card, index: 0 }, Duration::EndOfTurn);
    cantrip_api::Game::end_sentence(&mut cx);
    change(&mut cx, source, Color::Blue, Color::Green);
    cantrip_api::Game::damage(&mut cx, blue, Target::Player(YOU), 3);
    assert_eq!(cx.s().player(YOU).life, 23);
}
