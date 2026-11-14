//! Tower of Coireall / Joven's Tools: the lasting rule reads the words which
//! created it, while its second argument still identifies the affected attacker.
use super::can_block;
use crate::{Cx, Game, Setup};
use cantrip_api::*;

fn allowed(q: &dyn Query, on: ObjectId, blocker: ObjectId, attacker: ObjectId) -> bool {
    on != attacker || !q.chars(blocker).colors.has(q.words(on).color(Color::Red))
}
static SOURCE: CardDef = CardDef {
    name: "Lasting Block Text Source", types: TypeSet::of(&[CardType::Artifact]),
    lasting_block_rules: &[LastingBlockRule { text: "Can't be blocked by red creatures", allows: allowed }],
    ..CardDef::BLANK
};
const fn creature(name: &'static str, color: Color) -> CardDef {
    CardDef { name, colors: Some(ColorSet::of(color)), types: TypeSet::of(&[CardType::Creature]), power: Some(2), toughness: Some(2), ..CardDef::BLANK }
}
static RED: CardDef = creature("Lasting Block Red", Color::Red);
static BLUE: CardDef = creature("Lasting Block Blue", Color::Blue);
static GREEN: CardDef = creature("Lasting Block Green", Color::Green);

#[test]
fn block_restriction_keeps_source_words_instead_of_the_affected_creatures_words() {
    for changed_before in [false, true] {
        let cards = Box::leak(Box::new(CardTable::new([&SOURCE, &RED, &BLUE, &GREEN])));
        let mut game = Game::new(cards, &Setup { decks: vec![vec![], vec![]], ..Default::default() });
        game.state.update_turn(|t| t.number = 1);
        let mut put = |card, owner| {
            let id = game.state.push_obj(Obj::new(cards.id_of(card), owner, Zone::BATTLEFIELD));
            game.state.zone_push(Zone::BATTLEFIELD, id); id
        };
        let source = put(&SOURCE, PlayerId(0));
        let attacker = put(&RED, PlayerId(0));
        let unaffected = put(&RED, PlayerId(0));
        let red = put(&RED, PlayerId(1)); let blue = put(&BLUE, PlayerId(1)); let green = put(&GREEN, PlayerId(1));
        let mut cx = Cx { g: &mut game, frame: usize::MAX, cursor: 0, mana_source: None, entering: None, lineage: vec![], damage: None, asked_at: None, doer: None, producing: None };
        let change = |cx: &mut Cx, id, from, to| cantrip_api::Game::add_effect(cx, id, id,
            EffectKind::ChangeText { from: Word::Color(from), to: Word::Color(to) }, Duration::Indefinite);
        if changed_before { change(&mut cx, source, Color::Red, Color::Blue); }
        cantrip_api::Game::add_effect(&mut cx, source, attacker,
            EffectKind::BlockRestriction { card: cards.id_of(&SOURCE), index: 0 }, Duration::EndOfTurn);
        change(&mut cx, source, if changed_before { Color::Blue } else { Color::Red }, Color::Green);
        // A different text change on the target must never supply the rule's words.
        change(&mut cx, attacker, Color::Red, Color::White);
        assert_eq!(can_block(&cx, red, attacker, &[]), changed_before);
        assert_eq!(can_block(&cx, blue, attacker, &[]), !changed_before);
        assert!(can_block(&cx, green, attacker, &[]));
        assert!(can_block(&cx, red, unaffected, &[]), "second callback argument must remain the affected object");
    }
}
