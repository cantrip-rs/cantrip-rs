//! A card exiled "until [a permanent] leaves the battlefield" returns the moment that permanent leaves (CR 610.3), however
//! deep in a resolution, and an Aura returning this way chooses what it enchants as it enters (CR 303.4f): that question is
//! put to the position it arises in. The resolution can't stop there - the sacrifice that made the permanent leave has no
//! suspension to give back - so it goes on to the point it next asks, and is wound back to the question; on test-only cards.

use std::sync::OnceLock;

use cantrip_api::Game as Rules;
use cantrip_engine::*;

/// "{T}: Sacrifice this artifact. You gain 3 life. Choose a card in your hand."
fn give_way(g: &mut dyn Rules, item: &StackItem) -> Result<(), Suspend> {
    g.sacrifice(item.controller, &[item.object]);
    g.gain_life(item.controller, 3);
    let hand = g.state().zone(Zone::hand(item.controller)).to_vec();
    let eligible = hand.iter().copied().take(1).collect();
    g.ask(item.controller, RequestKind::choose_cards(item.object, Purpose::Reveal, hand, eligible, 1, false, ""))?;
    Ok(())
}

static WARDEN: CardDef = CardDef {
    name: "Probe Warden",
    types: TypeSet::of(&[CardType::Artifact]),
    abilities: &[ability(TAP, give_way, "{T}: Sacrifice this artifact. You gain 3 life. Choose a card in your hand.")],
    ..CardDef::BLANK
};

fn creature(q: &dyn Query, _: &Asker, t: Target) -> bool {
    matches!(t, Target::Object(id) if q.is_on_battlefield(id) && q.chars(id).is(CardType::Creature))
}

static CHARM: CardDef = CardDef {
    name: "Probe Charm",
    types: TypeSet::of(&[CardType::Enchantment]),
    subtypes: SubtypeSet::of(&[Subtype::Aura]),
    enchant: Some(target(creature)),
    ..CardDef::BLANK
};

static BEAR: CardDef = CardDef { name: "Probe Bear", types: TypeSet::of(&[CardType::Creature]), power: Some(2), toughness: Some(2), ..CardDef::BLANK };

static FILLER: CardDef = CardDef { name: "Filler", has_cost: false, types: TypeSet::of(&[CardType::Land]), ..CardDef::BLANK };

fn table() -> &'static CardTable {
    static TABLE: OnceLock<CardTable> = OnceLock::new();
    TABLE.get_or_init(|| CardTable::new([&WARDEN, &CHARM, &BEAR, &FILLER]))
}

fn put(game: &mut Game, def: &'static CardDef, owner: PlayerId, zone: Zone) -> ObjectId {
    let id = game.state.push_obj(Obj::new(table().id_of(def), owner, zone));
    let timestamp = game.state.timestamp();
    game.state.update_obj(id, |o| o.timestamp = timestamp);
    game.state.zone_push(zone, id);
    id
}

/// The first seat's Probe Warden holds the second seat's Probe Charm, exiled from the battlefield until the Warden leaves it;
/// the second seat has two Probe Bears for it to come back on. The first seat activates the Warden: the Charm is back the
/// moment the Warden is sacrificed, its owner choosing the second Bear while the first seat has not yet gained the 3 life
/// the resolution goes on to give; the resolution then goes on with the answer.
#[test]
fn a_returning_auras_question_is_put_to_the_position_it_arises_in() {
    let filler = table().find("Filler").unwrap();
    let mut game = Game::new(table(), &Setup { decks: vec![vec![filler; 30], vec![filler; 30]], seed: 1, shuffle: false, opening_chooser: None, ..Default::default() });
    game.verify = true;
    let (you, them) = (PlayerId(0), PlayerId(1));
    let warden = put(&mut game, &WARDEN, you, Zone::BATTLEFIELD);
    let bears = [put(&mut game, &BEAR, them, Zone::BATTLEFIELD), put(&mut game, &BEAR, them, Zone::BATTLEFIELD)];
    let charm = put(&mut game, &CHARM, them, Zone::exile(them));
    let text = game.state.obj(warden).card;
    game.state.update_obj(charm, |o| o.exiled = Some(ExileLink { by: warden, text, acquired: Acquired::OWN, from: Zone::BATTLEFIELD, until_leaves: true, return_by: None, encoded: false }));
    let mut activated = false;
    let mut asked_host = None;
    for _ in 0..100 {
        game.run();
        let Some(request) = game.pending().cloned() else { panic!("the game ended") };
        if activated && matches!(request.kind, RequestKind::Priority { .. }) && game.state.stack.is_empty() {
            break;
        }
        let answer = match &request.kind {
            RequestKind::Priority { options } if request.player == you && !activated => {
                let option = options.iter().find(|o| o.kind == ActionKind::ActivateAbility && o.source == Some(warden));
                match option {
                    Some(option) => {
                        activated = true;
                        Answer::Act(Action::of(option))
                    }
                    None => Answer::Pass,
                }
            }
            RequestKind::Priority { .. } => Answer::Pass,
            RequestKind::ChooseTarget { candidates, .. } => {
                // The position the question arises in: the Warden gone, the Charm still in exile, no life gained yet.
                asked_host = Some((game.state.player(you).life, game.state.battlefield().contains(&warden), candidates.clone()));
                Answer::Targets(vec![Target::Object(bears[1])])
            }
            RequestKind::ChooseCards { .. } => Answer::Cards(first_choices(&game, &request)),
            other => panic!("unexpected question {other:?}"),
        };
        game.answer(request.id, request.player, answer).unwrap();
    }
    let (life, warden_there, candidates) = asked_host.expect("the Charm's owner chose what it enchants");
    assert_eq!(life, 20, "the question waits where the Warden left, before the resolution gains the life");
    assert!(!warden_there, "the Warden had been sacrificed");
    assert_eq!(candidates, vec![Target::Object(bears[0]), Target::Object(bears[1])]);
    assert_eq!(game.state.player(you).life, 23, "the resolution went on with the answer");
    let back = game.state.battlefield().iter().copied().find(|&id| game.name(id) == "Probe Charm").expect("the Charm is back");
    assert_eq!(game.state.obj(back).attached_to, Some(bears[1]));
    assert_eq!(game.state.obj(back).controller, them);
}
