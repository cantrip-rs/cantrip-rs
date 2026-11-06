//! "A source of your choice" may be an object a delayed triggered ability waiting to trigger refers to, even one no longer
//! where it was (CR 609.7a) - and whether the ability refers to its own source is read from its words
//! (`CardTable::names_its_source`). Dragonhawk's "At the beginning of your next end step, Dragonhawk deals 2 damage to each
//! opponent ..." names it, so once Dragonhawk has died its waiting ability still makes it a choice, and a "next time" shield
//! from it prevents that end-step damage; Flickerwisp's "Return that card to the battlefield ..." names only the card it
//! exiled, so a dead Flickerwisp is no choice while the card is. No printed card in the engine chooses "a source of your
//! choice" yet, so test cards do here: the printed Dragonhawk and Flickerwisp, with every card the engine has, beside "Probe
//! Doom", which kills the creature first - so that no spell on the stack refers to it as the source is chosen - and "Probe
//! Palm", which then chooses.

use std::sync::OnceLock;

use cantrip_api::Game as Rules;
use cantrip_engine::*;
use cantrip_mechanics::prevention;

fn creature(q: &dyn Query, _: &Asker, t: Target) -> bool {
    matches!(t, Target::Object(id) if q.is_on_battlefield(id) && q.chars(id).is(CardType::Creature))
}

/// "Destroy target creature."
fn doom(g: &mut dyn Rules, item: &StackItem) -> Result<(), Suspend> {
    if let Target::Object(id) = item.target(0)
        && g.target_legal(item, 0)
    {
        g.destroy(id);
    }
    Ok(())
}

/// A zero-cost instant: destroy target creature.
static DOOM: CardDef = CardDef { name: "Probe Doom", types: TypeSet::of(&[CardType::Instant]), spell: spell(&[target(creature)], doom), ..CardDef::BLANK };

/// "The next time a source of your choice would deal damage to you this turn, prevent that damage."
fn palm(g: &mut dyn Rules, item: &StackItem) -> Result<(), Suspend> {
    prevention::next_time_chosen_source_would_damage_you(g, item, 0)?;
    Ok(())
}

/// A zero-cost instant: the next time a source of your choice would deal damage to you this turn, prevent that damage.
static PALM: CardDef = CardDef {
    name: "Probe Palm",
    types: TypeSet::of(&[CardType::Instant]),
    spell: spell(&[], palm),
    granted_preventions: &[prevention::FROM_CHOSEN_SOURCE.text("The next time a source of your choice would deal damage to you this turn, prevent that damage.")],
    ..CardDef::BLANK
};

fn table() -> &'static CardTable {
    static TABLE: OnceLock<CardTable> = OnceLock::new();
    TABLE.get_or_init(|| CardTable::new(cantrip_cards::all().into_iter().chain([&DOOM, &PALM])))
}

/// What Bot2 was offered as Probe Palm resolved, and the game once Bot1's first turn is over.
struct Played {
    game: Game,
    /// The creature Bot1 cast, as the object it was on the battlefield (the source of its delayed ability).
    creature: ObjectId,
    offered: Vec<ObjectId>,
}

/// Bot1 casts `moxen` of `mox` and `name` on turn 1 (Flickerwisp exiling one of the Moxen). Once the creature's own ability
/// has resolved, Bot2 kills it with Probe Doom, then casts Probe Palm and chooses `choose(offered)` as the source. Played to
/// the start of turn 2.
fn played(mox: &str, moxen: usize, name: &str, choose: impl Fn(&Game, &[ObjectId], ObjectId) -> ObjectId) -> Played {
    let find = |n: &str| table().find(n).unwrap_or_else(|| panic!("no card {n}"));
    let mut first: Vec<CardId> = std::iter::repeat_n(find(mox), moxen).collect();
    first.push(find(name));
    first.extend(std::iter::repeat_n(find("Mountain"), 30));
    let mut second = vec![find("Probe Doom"), find("Probe Palm")];
    second.extend(std::iter::repeat_n(find("Island"), 30));
    let mut game = Game::new(table(), &Setup { decks: vec![first, second], seed: 1, shuffle: false, opening_chooser: None, ..Default::default() });
    game.verify = true;
    let on_battlefield = |game: &Game| game.state.battlefield().iter().copied().find(|&id| game.name(id) == name);
    let (mut creature, mut palm, mut offered) = (None, false, None);
    for _ in 0..400 {
        game.run();
        if game.state.turn.number >= 2 {
            break;
        }
        let request = game.pending().cloned().expect("the game goes on");
        let castable = |options: &[ActionOption], n: &str| options.iter().find(|o| o.kind == ActionKind::CastSpell && o.source.is_some_and(|s| game.name(s) == n)).cloned();
        let answer = match &request.kind {
            RequestKind::Priority { options } if request.player == PlayerId(0) => {
                match castable(options, mox).or_else(|| castable(options, name).filter(|_| game.state.stack.is_empty())) {
                    Some(option) => Answer::Act(Action::of(&option)),
                    None => Answer::Pass,
                }
            }
            RequestKind::Priority { options } if game.state.stack.is_empty() && creature.is_none() && on_battlefield(&game).is_some() => {
                creature = on_battlefield(&game);
                let mut action = Action::of(&castable(options, "Probe Doom").expect("Probe Doom"));
                action.targets = vec![Target::Object(creature.unwrap())];
                Answer::Act(action)
            }
            RequestKind::Priority { options } if game.state.stack.is_empty() && creature.is_some() && !palm => {
                palm = true;
                Answer::Act(Action::of(&castable(options, "Probe Palm").expect("Probe Palm")))
            }
            RequestKind::Priority { .. } => Answer::Pass,
            // Flickerwisp's "exile another target permanent": one of the Moxen.
            RequestKind::ChooseTarget { candidates, .. } => {
                let mox = candidates.iter().copied().find(|t| matches!(t, Target::Object(id) if game.name(*id) == mox)).expect("a Mox to exile");
                Answer::Targets(vec![mox])
            }
            RequestKind::ChooseCards { purpose: Purpose::Choose, cards, .. } if request.player == PlayerId(1) => {
                offered = Some(cards.clone());
                Answer::Cards(vec![choose(&game, cards, creature.expect("the creature Probe Doom killed"))])
            }
            RequestKind::ChooseCostPayment { choices, .. } => Answer::CostPayment(choices.iter().position(|c| c.kind == CostPaymentKind::Mana).unwrap_or(0)),
            RequestKind::PayCosts { options, remaining, mana_abilities, .. } => {
                let mox = options.iter().find(|o| o.kind == ActionKind::ActivateAbility && o.source.is_some_and(|s| game.name(s) == mox));
                super::pay(options, remaining, *mana_abilities, mox)
            }
            RequestKind::DiscardToHandSize { count, hand } => Answer::Cards(hand.iter().copied().take(*count as usize).collect()),
            RequestKind::DeclareAttackers { .. } => Answer::Attack(Vec::new()),
            RequestKind::DeclareBlockers { .. } => Answer::Block(Vec::new()),
            other => panic!("unexpected question {other:?}"),
        };
        game.answer(request.id, request.player, answer).unwrap();
    }
    assert!(palm, "Bot2 never cast Probe Palm");
    assert!(on_battlefield(&game).is_none(), "{name} was not destroyed");
    Played { game, creature: creature.expect("the creature"), offered: offered.expect("Bot2 was never asked for a source") }
}

/// Dragonhawk died, and its waiting end-step ability names it: it is a source to choose, and the shield from it prevents the
/// 2 damage that ability deals Bot2 at Bot1's end step (one card still exiled).
#[test]
fn a_waiting_delayed_ability_that_names_its_dead_source_offers_it() {
    let Played { game, creature, .. } = played("Mox Ruby", 5, "Dragonhawk, Fate's Tempest", |game, offered, dragonhawk| {
        assert!(offered.contains(&dragonhawk), "the dead Dragonhawk its waiting ability names is no choice: {offered:?}");
        assert!(!game.state.zone(Zone::BATTLEFIELD).contains(&dragonhawk) && !game.state.stack.iter().any(|i| i.object == dragonhawk));
        dragonhawk
    });
    assert_eq!(game.state.player(PlayerId(1)).life, 20, "the end-step damage from the chosen Dragonhawk was dealt");
    let prevented = game.log.iter().any(|r| matches!(r.event, Event::DamagePrevented { source, target: Target::Player(PlayerId(1)), amount: 2, .. } if source == creature));
    assert!(prevented, "no record of the Dragonhawk's 2 damage prevented");
}

/// Flickerwisp died, and its waiting "return that card" ability names the exiled Mox, not Flickerwisp: the Mox is a source to
/// choose, the dead Flickerwisp - as it was, or the card in the graveyard - is not.
#[test]
fn a_waiting_delayed_ability_that_does_not_name_its_dead_source_does_not_offer_it() {
    let Played { game, creature, offered } = played("Mox Pearl", 3, "Flickerwisp", |game, offered, _| {
        *offered.iter().find(|&&id| game.name(id) == "Mox Pearl" && game.state.obj(id).zone.is(ZoneKind::Exile)).expect("the exiled Mox its ability returns")
    });
    assert!(!offered.contains(&creature), "the dead Flickerwisp, which its waiting ability never names, was offered");
    assert!(!offered.iter().any(|&id| game.name(id) == "Flickerwisp"), "a Flickerwisp was offered: {offered:?}");
}
