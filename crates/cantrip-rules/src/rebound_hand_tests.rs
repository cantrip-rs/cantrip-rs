//! CR 702.88a, 109.5 and 400.3: rebound requires the resolving spell controller's hand.
//! Ordered decks and real casts, including an effect casting another player's card from hand; no seed search.
//! The shared keyword is used by Ephemerate and World at War. The test cast effect isolates the foreign-hand rule.
use crate::{Game, Setup};
use cantrip_api::*;

const YOU: PlayerId = PlayerId(0);
const THEM: PlayerId = PlayerId(1);

fn gain(g: &mut dyn cantrip_api::Game, item: &StackItem) -> Result<(), Suspend> {
    g.gain_life(item.controller, 1);
    Ok(())
}
static REBOUND: CardDef = CardDef {
    name: "Hand Test Rebound",
    types: TypeSet::of(&[CardType::Sorcery]),
    spell: spell(&[], gain),
    rebound: true,
    ..CardDef::BLANK
};
static FILLER: CardDef = CardDef {
    name: "Hand Test Filler",
    has_cost: false,
    types: TypeSet::of(&[CardType::Land]),
    ..CardDef::BLANK
};
fn cast_from(g: &mut dyn cantrip_api::Game, item: &StackItem, owner: PlayerId) -> Result<(), Suspend> {
    let card = g.state().zone(Zone::hand(owner)).iter().copied()
        .find(|&id| g.card(g.obj(id).card).rebound).expect("the ordered rebound card");
    assert!(g.cast_during_resolution_as(card, CastTerms { caster: Some(item.controller), instead: Some(ManaCost::ZERO), ..CastTerms::default() })?);
    Ok(())
}
fn own(g: &mut dyn cantrip_api::Game, item: &StackItem) -> Result<(), Suspend> {
    cast_from(g, item, item.controller)
}
fn other(g: &mut dyn cantrip_api::Game, item: &StackItem) -> Result<(), Suspend> {
    cast_from(g, item, THEM)
}
static OWN: CardDef = CardDef {
    name: "Hand Test Own Cast",
    types: TypeSet::of(&[CardType::Sorcery]),
    spell: spell(&[], own),
    ..CardDef::BLANK
};
static OTHER: CardDef = CardDef {
    name: "Hand Test Other Cast",
    types: TypeSet::of(&[CardType::Sorcery]),
    spell: spell(&[], other),
    ..CardDef::BLANK
};
fn grant_itself_rebound(g: &mut dyn cantrip_api::Game, item: &StackItem) -> Result<(), Suspend> {
    g.gain_life(item.controller, 1);
    g.add_effect(item.object, item.object, EffectKind::AddKeywords(Keywords::of(&[Keyword::Rebound])), Duration::Indefinite);
    Ok(())
}
static GRANTED: CardDef = CardDef {
    name: "Hand Test Granted",
    types: TypeSet::of(&[CardType::Sorcery]),
    spell: spell(&[], grant_itself_rebound),
    ..CardDef::BLANK
};

/// Answers `g`'s questions - casting `cast` from the first player's hand once, passing otherwise - until `done`.
fn drive(g: &mut Game, table: &CardTable, cast: &'static CardDef, done: &dyn Fn(&Game) -> bool) {
    let mut announced = false;
    for _ in 0..1000 {
        g.run();
        if done(g) {
            return;
        }
        let r = g.pending().cloned().expect("question");
        let answer = match &r.kind {
            RequestKind::Priority { options } if r.player == YOU && !announced => {
                if let Some(o) = options.iter().find(|o| o.kind == ActionKind::CastSpell && o.source.is_some_and(|id| g.state.obj(id).card == table.id_of(cast))) {
                    announced = true;
                    Answer::Act(Action::of(o))
                } else { Answer::Pass }
            }
            RequestKind::Priority { .. } => Answer::Pass,
            RequestKind::DeclareAttackers { .. } => Answer::Attack(Vec::new()),
            RequestKind::DeclareBlockers { .. } => Answer::Block(Vec::new()),
            RequestKind::DiscardToHandSize { count, hand } => Answer::Cards(hand.iter().copied().take(*count as usize).collect()),
            other => panic!("unexpected question {other:?}"),
        };
        g.answer(r.id, r.player, answer).unwrap();
    }
    panic!("the spell never resolved");
}

fn resolved(owner: PlayerId) -> Game {
    let table = Box::leak(Box::new(CardTable::new([&REBOUND, &FILLER, &OWN, &OTHER])));
    let cast = if owner == YOU { &OWN } else { &OTHER };
    let mut decks = vec![vec![table.id_of(&FILLER); 35]; 2];
    decks[0][0] = table.id_of(cast);
    decks[owner.0 as usize][1] = table.id_of(&REBOUND);
    let mut g = Game::new(table, &Setup { decks, ..Setup::default() });
    drive(&mut g, table, cast, &|g| g.state.player(YOU).life == 21 && g.state.stack.is_empty());
    let history = g.history();
    let cast = history.casts().find(|c| g.state.obj(c.spell).card == table.id_of(&REBOUND)).expect("rebound really cast");
    assert_eq!(cast.player, YOU);
    assert_eq!(cast.from, ZoneKind::Hand);
    assert_eq!(g.state.obj(cast.spell).owner, owner);
    g
}

#[test]
fn own_hand_cast_without_paying_still_rebounds() {
    let g = resolved(YOU);
    assert_eq!(g.state.zone(Zone::exile(YOU)).len(), 1);
    assert_eq!(g.state.delayed.len(), 1);
    assert_eq!(g.state.zone(Zone::graveyard(YOU)).len(), 1, "only the cast effect went to graveyard");
}

#[test]
fn foreign_hand_cast_resolves_to_owner_graveyard_without_rebound() {
    let g = resolved(THEM);
    assert!(g.state.zone(Zone::exile(THEM)).is_empty(), "foreign-hand cast must not rebound");
    assert!(g.state.delayed.is_empty(), "no next-upkeep cast offer");
    assert_eq!(g.state.zone(Zone::graveyard(THEM)).len(), 1);
}

/// "Gains rebound" (CR 702.88a, 613.1f): a spell with no rebound of its own is given it by an effect while it is on the stack
/// - the spell here gives it to itself as it resolves - and, cast from its controller's hand, it is exiled instead of put into
/// the graveyard, with the delayed ability to cast it at the next upkeep.
#[test]
fn granted_rebound_exiles_the_spell_cast_from_hand() {
    let table = Box::leak(Box::new(CardTable::new([&FILLER, &GRANTED])));
    let mut decks = vec![vec![table.id_of(&FILLER); 35]; 2];
    decks[0][0] = table.id_of(&GRANTED);
    let mut g = Game::new(table, &Setup { decks, ..Setup::default() });
    drive(&mut g, table, &GRANTED, &|g| g.state.player(YOU).life == 21 && g.state.stack.is_empty());
    assert_eq!(g.state.zone(Zone::exile(YOU)).len(), 1, "exiled with the rebound it was given");
    assert_eq!(g.state.delayed.len(), 1, "its next-upkeep cast offer");
    assert!(g.state.zone(Zone::graveyard(YOU)).is_empty());
}
