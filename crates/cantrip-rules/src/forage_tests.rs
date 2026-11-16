//! "Whenever you forage" (CR 701.61a: "to forage means 'exile three cards from your graveyard or sacrifice a Food'"), over
//! test cards. The watcher gains 1 life whenever its controller forages, so its life counts the forages, by either way, as
//! a cost of a spell or as a resolving ability asks.

use crate::{Game, Setup};
use cantrip_api::*;
use cantrip_mechanics::forage::{self, EXILE_THREE_CARDS, SACRIFICE_A_FOOD, YOU_FORAGE};
use cantrip_state::{Frame, FrameKind};

const YOU: PlayerId = PlayerId(0);
const THEM: PlayerId = PlayerId(1);

fn nothing(_: &mut dyn cantrip_api::Game, _: &StackItem) -> Result<(), Suspend> {
    Ok(())
}

fn gain_one(g: &mut dyn cantrip_api::Game, item: &StackItem) -> Result<(), Suspend> {
    g.gain_life(item.controller, 1);
    Ok(())
}

/// "You may forage. You may forage.": two forages in one resolution.
fn forage_twice(g: &mut dyn cantrip_api::Game, item: &StackItem) -> Result<(), Suspend> {
    forage::may_forage(g, item)?;
    forage::may_forage(g, item)?;
    Ok(())
}

/// "You may forage. If you do, you gain 5 life.": the "if you do" reads whether `may_forage` foraged.
fn forage_then_gain_five(g: &mut dyn cantrip_api::Game, item: &StackItem) -> Result<(), Suspend> {
    if forage::may_forage(g, item)? {
        g.gain_life(item.controller, 5);
    }
    Ok(())
}

static FILLER: CardDef = CardDef { name: "Forage Test Filler", has_cost: false, types: TypeSet::of(&[CardType::Land]), ..CardDef::BLANK };

/// "Whenever you forage, you gain 1 life."
static WATCHER: CardDef = CardDef {
    name: "Forage Test Watcher",
    types: TypeSet::of(&[CardType::Enchantment]),
    abilities: &[triggered(YOU_FORAGE, gain_one, "Whenever you forage, you gain 1 life.")],
    ..CardDef::BLANK
};

static FOOD: CardDef = CardDef { name: "Forage Test Food", types: TypeSet::of(&[CardType::Artifact]), subtypes: SubtypeSet::of(&[Subtype::Food]), ..CardDef::BLANK };

/// "As an additional cost to cast this spell, forage."
static FEAST: CardDef = CardDef {
    name: "Forage Test Feast",
    types: TypeSet::of(&[CardType::Sorcery]),
    additional_choice: &[EXILE_THREE_CARDS, SACRIFICE_A_FOOD],
    spell: spell(&[], nothing),
    ..CardDef::BLANK
};

/// "{0}: You may forage. You may forage."
static FORAGER: CardDef = CardDef {
    name: "Forage Test Forager",
    types: TypeSet::of(&[CardType::Artifact]),
    abilities: &[ability(of_mana(mana("{0}")), forage_twice, "{0}: You may forage. You may forage.")],
    ..CardDef::BLANK
};

/// "{0}: You may forage. If you do, you gain 5 life."
static PROVIDER: CardDef = CardDef {
    name: "Forage Test Provider",
    types: TypeSet::of(&[CardType::Artifact]),
    abilities: &[ability(of_mana(mana("{0}")), forage_then_gain_five, "{0}: You may forage. If you do, you gain 5 life.")],
    ..CardDef::BLANK
};

/// Your precombat main phase with priority, nothing anywhere.
fn game() -> Game {
    let table: &'static CardTable = Box::leak(Box::new(CardTable::new([&FILLER, &WATCHER, &FOOD, &FEAST, &FORAGER, &GATHERER, &PROVIDER])));
    let deck = || vec![table.id_of(&FILLER); 20];
    let mut g = Game::new(table, &Setup { decks: vec![deck(), deck()], ..Default::default() });
    while !g.state.agenda.is_empty() {
        g.state.frame_remove(0);
    }
    g.state.update_turn(|t| {
        t.active = YOU;
        t.priority = YOU;
        t.set_step(Step::PrecombatMain);
    });
    g.state.frame_push(Frame::new(FrameKind::Priority));
    g.state.update_player(YOU, |p| p.turn_began = 1);
    g
}

fn put(g: &mut Game, def: &'static CardDef, owner: PlayerId, zone: Zone) -> ObjectId {
    let id = g.state.push_obj(Obj::new(g.cards.id_of(def), owner, zone));
    g.state.zone_push(zone, id);
    refresh(g);
    id
}

/// The priority question already asked is asked again, of the changed game.
fn refresh(g: &mut Game) {
    if let Some(at) = g.state.agenda.len().checked_sub(1)
        && g.state.agenda[at].pending.as_ref().is_some_and(|r| matches!(r.kind, RequestKind::Priority { .. }))
    {
        g.state.update_frame(at, |f| f.pending = None);
    }
}

fn next(g: &mut Game) -> Request {
    g.run();
    g.pending().cloned().expect("a question")
}

/// Takes `source`'s first option at priority and lets everything on the stack resolve: each "may" answered `yes`, an
/// additional cost chosen as `way` (an index into the spell's choices), cards chosen as the first that will do.
fn play(g: &mut Game, source: ObjectId, way: u8, yes: bool) {
    let r = next(g);
    let RequestKind::Priority { options } = &r.kind else { panic!("expected priority, got {:?}", r.kind) };
    let option = options.iter().find(|o| o.source == Some(source) && o.index == 0).expect("it is offered").clone();
    g.answer(r.id, r.player, Answer::Act(Action::of(&option))).expect("a legal answer");
    for _ in 0..40 {
        let r = next(g);
        match &r.kind {
            RequestKind::ChooseModes { additional_cost: true, .. } => g.answer(r.id, r.player, Answer::Modes(vec![way])).expect("a way"),
            RequestKind::PayCosts { options, .. } => {
                let pay = options.iter().find(|o| matches!(o.kind, ActionKind::ConfirmCasting | ActionKind::AssignMana)).expect("it pays").clone();
                g.answer(r.id, r.player, Answer::Act(Action::of(&pay))).expect("a legal answer");
            }
            RequestKind::ChooseCards { cards, .. } => {
                let cards = cards.clone();
                if g.answer(r.id, r.player, Answer::Cards(cards.iter().copied().take(3).collect())).is_err() {
                    g.answer(r.id, r.player, Answer::Cards(cards[..1].to_vec())).expect("one card");
                }
            }
            RequestKind::UnlessPays { options, .. } => {
                let kind = if yes { ActionKind::PayCost } else { ActionKind::DeclineCost };
                let pick = options.iter().find(|o| o.kind == kind).expect("pay or decline").clone();
                g.answer(r.id, r.player, Answer::Act(Action::of(&pick))).expect("a legal answer");
            }
            RequestKind::May { .. } => g.answer(r.id, r.player, if yes { Answer::Yes } else { Answer::No }).expect("an answer"),
            RequestKind::OrderTriggers { sources, .. } => g.answer(r.id, r.player, Answer::Order((0..sources.len() as u32).collect())).expect("an order"),
            RequestKind::Priority { .. } if g.state.stack.is_empty() => return,
            _ => g.answer(r.id, r.player, Answer::Pass).unwrap_or_else(|e| panic!("{e:?} for {:?}", r.kind)),
        }
    }
    panic!("the stack never emptied");
}

fn life(g: &Game, player: PlayerId) -> i32 {
    g.state.player(player).life
}

fn count(g: &Game, zone: Zone) -> usize {
    g.state.zone(zone).len()
}

/// Forage as a spell's additional cost, by exiling three cards from the graveyard, is a forage.
#[test]
fn an_additional_cost_by_the_graveyard_is_a_forage() {
    let mut g = game();
    put(&mut g, &WATCHER, YOU, Zone::BATTLEFIELD);
    for _ in 0..3 {
        put(&mut g, &FILLER, YOU, Zone::graveyard(YOU));
    }
    let feast = put(&mut g, &FEAST, YOU, Zone::hand(YOU));
    let start = life(&g, YOU);
    play(&mut g, feast, 0, true);
    assert_eq!(count(&g, Zone::exile(YOU)), 3, "three exiled");
    assert_eq!(life(&g, YOU), start + 1);
}

/// Forage as a spell's additional cost, by sacrificing a Food, is a forage.
#[test]
fn an_additional_cost_by_a_food_is_a_forage() {
    let mut g = game();
    put(&mut g, &WATCHER, YOU, Zone::BATTLEFIELD);
    let food = put(&mut g, &FOOD, YOU, Zone::BATTLEFIELD);
    let feast = put(&mut g, &FEAST, YOU, Zone::hand(YOU));
    let start = life(&g, YOU);
    play(&mut g, feast, 1, true);
    assert!(!g.state.zone(Zone::BATTLEFIELD).contains(&food), "the Food was sacrificed");
    assert_eq!(life(&g, YOU), start + 1);
}

/// "You may forage" twice as an ability resolves is two forages, by the graveyard and then the Food; an opponent's watcher
/// sees neither, and declining is no forage.
#[test]
fn foraging_twice_as_an_effect_triggers_twice() {
    let mut g = game();
    put(&mut g, &WATCHER, YOU, Zone::BATTLEFIELD);
    put(&mut g, &WATCHER, THEM, Zone::BATTLEFIELD);
    for _ in 0..3 {
        put(&mut g, &FILLER, YOU, Zone::graveyard(YOU));
    }
    put(&mut g, &FOOD, YOU, Zone::BATTLEFIELD);
    let forager = put(&mut g, &FORAGER, YOU, Zone::BATTLEFIELD);
    let (you, them) = (life(&g, YOU), life(&g, THEM));
    play(&mut g, forager, 0, true);
    assert_eq!(count(&g, Zone::graveyard(YOU)), 1, "three exiled, then the Food sacrificed");
    assert_eq!((life(&g, YOU), life(&g, THEM)), (you + 2, them));
    play(&mut g, forager, 0, true);
    assert_eq!(life(&g, YOU), you + 2, "nothing left to forage with");
}

/// Declining to forage is no forage.
#[test]
fn declining_is_no_forage() {
    let mut g = game();
    put(&mut g, &WATCHER, YOU, Zone::BATTLEFIELD);
    put(&mut g, &FOOD, YOU, Zone::BATTLEFIELD);
    let forager = put(&mut g, &FORAGER, YOU, Zone::BATTLEFIELD);
    let start = life(&g, YOU);
    play(&mut g, forager, 0, false);
    assert_eq!(life(&g, YOU), start);
}

/// "Forage: Nothing.": forage as an activated ability's cost is a forage, as it is paid, before the ability resolves.
static GATHERER: CardDef = CardDef {
    name: "Forage Test Gatherer",
    types: TypeSet::of(&[CardType::Artifact]),
    abilities: &[ability(SACRIFICE_A_FOOD.cost, nothing, "Forage: Nothing.")],
    ..CardDef::BLANK
};

#[test]
fn an_activated_abilitys_cost_is_a_forage() {
    let mut g = game();
    put(&mut g, &WATCHER, YOU, Zone::BATTLEFIELD);
    put(&mut g, &FOOD, YOU, Zone::BATTLEFIELD);
    let gatherer = put(&mut g, &GATHERER, YOU, Zone::BATTLEFIELD);
    let start = life(&g, YOU);
    play(&mut g, gatherer, 0, true);
    assert_eq!(life(&g, YOU), start + 1);
}

/// "If you do" (CR 608.2c) follows the forage: foraging by the graveyard and then by a Food each gain 5, declining gains
/// nothing, and with nothing left to forage with there is no forage to follow.
#[test]
fn if_you_do_follows_whether_you_foraged() {
    let mut g = game();
    for _ in 0..3 {
        put(&mut g, &FILLER, YOU, Zone::graveyard(YOU));
    }
    put(&mut g, &FOOD, YOU, Zone::BATTLEFIELD);
    let provider = put(&mut g, &PROVIDER, YOU, Zone::BATTLEFIELD);
    let start = life(&g, YOU);
    play(&mut g, provider, 0, false);
    assert_eq!(life(&g, YOU), start, "declined");
    play(&mut g, provider, 0, true);
    assert_eq!((life(&g, YOU), count(&g, Zone::graveyard(YOU))), (start + 5, 0), "three cards exiled");
    play(&mut g, provider, 0, true);
    assert_eq!((life(&g, YOU), count(&g, Zone::graveyard(YOU))), (start + 10, 1), "the Food sacrificed");
    play(&mut g, provider, 0, true);
    assert_eq!(life(&g, YOU), start + 10, "nothing left to forage with");
}
