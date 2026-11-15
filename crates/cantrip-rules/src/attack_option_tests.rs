//! Optional costs to attack (CR 508.1g-j): enlist (CR 702.154) and "you may exert this creature as it attacks" (CR 701.43d),
//! each a static ability kept with the "when you do" ability linked to it (CR 607.2h, `TriggeredDef::attack_option`). Chosen
//! once the attackers are chosen and tapped, locked in with the other costs to attack and paid with them, a declaration whose
//! optional cost can't be paid taken back. Over test cards; `cantrip_mechanics::enlist` and `exert` are what printed cards use.

use crate::{Game, Setup};
use cantrip_api::*;
use cantrip_state::{Frame, FrameKind};

const YOU: PlayerId = PlayerId(0);
const THEM: PlayerId = PlayerId(1);

/// Enlist's linked ability: "this creature gets +X/+0 until end of turn, where X is the tapped creature's power" - its power
/// as the ability resolves (CR 608.2h), and 0 for a negative one (CR 107.1b).
fn plus_x(g: &mut dyn cantrip_api::Game, item: &StackItem) -> Result<(), Suspend> {
    let Target::Object(tapped) = item.subject else { return Ok(()) };
    let x = g.chars(tapped).power.unwrap_or(0).max(0);
    if g.is_on_battlefield(item.object) {
        g.add_effect(item.object, item.object, EffectKind::Pump(x, 0), Duration::EndOfTurn);
    }
    Ok(())
}

fn gain_one(g: &mut dyn cantrip_api::Game, item: &StackItem) -> Result<(), Suspend> {
    g.gain_life(item.controller, 1);
    Ok(())
}

const ENLIST: AbilityDef = paid_as_it_attacks(AttackOption::Enlist, plus_x, "Enlist");

/// "Whenever this creature enlists a creature" (CR 702.154c).
fn this_enlists(_: &dyn Query, source: ObjectId, event: &Event) -> Option<Fired> {
    match *event {
        Event::PaidToAttack { attacker, option: AttackOption::Enlist, .. } if attacker == source => Fired::PLAIN,
        _ => None,
    }
}

static FILLER: CardDef = CardDef { name: "Attack Option Filler", has_cost: false, types: TypeSet::of(&[CardType::Land]), ..CardDef::BLANK };
const CREATURE: TypeSet = TypeSet::of(&[CardType::Creature]);
/// Enlist, and "whenever this creature enlists a creature, you gain 1 life".
static RECRUITER: CardDef = CardDef {
    name: "Attack Option Recruiter",
    types: CREATURE,
    power: Some(2),
    toughness: Some(2),
    abilities: &[ENLIST, triggered(when(EventMask::PAID_TO_ATTACK, this_enlists), gain_one, "Whenever this creature enlists a creature, you gain 1 life.")],
    ..CardDef::BLANK
};
/// Enlist twice: two instances, each its own cost and trigger (CR 702.154d).
static DOUBLE: CardDef = CardDef { name: "Attack Option Double", types: CREATURE, power: Some(1), toughness: Some(1), abilities: &[ENLIST, ENLIST], ..CardDef::BLANK };
static BEAR: CardDef = CardDef { name: "Attack Option Bear", types: CREATURE, power: Some(3), toughness: Some(3), ..CardDef::BLANK };
fn plus_two(g: &mut dyn cantrip_api::Game, item: &StackItem) -> Result<(), Suspend> {
    if g.is_on_battlefield(item.object) {
        g.add_effect(item.object, item.object, EffectKind::Pump(2, 0), Duration::EndOfTurn);
    }
    Ok(())
}

fn nothing(_: &mut dyn cantrip_api::Game, _: &StackItem) -> Result<(), Suspend> {
    Ok(())
}

/// "{0}: This creature gets +2/+0 until end of turn." "Sacrifice this creature: Nothing happens."
static PUMPER: CardDef = CardDef {
    name: "Attack Option Pumper",
    types: CREATURE,
    power: Some(3),
    toughness: Some(3),
    abilities: &[
        ability(cost("{0}"), plus_two, "{0}: This creature gets +2/+0 until end of turn."),
        ability(SACRIFICE_THIS, nothing, "Sacrifice this creature: Nothing happens."),
    ],
    ..CardDef::BLANK
};
static VIGILANT: CardDef =
    CardDef { name: "Attack Option Vigilant", types: CREATURE, power: Some(4), toughness: Some(4), keywords: Keywords::of(&[Keyword::Vigilance]), ..CardDef::BLANK };
static HASTY: CardDef = CardDef { name: "Attack Option Hasty", types: CREATURE, power: Some(1), toughness: Some(1), keywords: Keywords::of(&[Keyword::Haste]), ..CardDef::BLANK };
/// "{T}: Add {G}."
static ELF: CardDef =
    CardDef { name: "Attack Option Elf", types: CREATURE, power: Some(1), toughness: Some(1), abilities: &[mana_ability(TAP, add(Mana::G), "{T}: Add {G}.")], ..CardDef::BLANK };
/// "You may exert this creature as it attacks. When you do, you gain 1 life."
static EXERTER: CardDef = CardDef {
    name: "Attack Option Exerter",
    types: CREATURE,
    power: Some(2),
    toughness: Some(2),
    abilities: &[paid_as_it_attacks(AttackOption::Exert, gain_one, "You may exert this creature as it attacks. When you do, you gain 1 life.")],
    ..CardDef::BLANK
};

/// A static permission, not an intervening trigger condition.
fn at_twenty(q: &dyn Query, me: &Asker) -> bool {
    q.state().player(me.controller).life == 20
        && !q.history().this_turn().iter().any(|r| matches!(r.event, Event::Exerted { permanent, .. } if permanent == me.source))
}
static CONDITIONAL: CardDef = CardDef {
    name: "Attack Option Conditional",
    abilities: &[paid_as_it_attacks(AttackOption::Exert, gain_one, "At life 20, you may exert it as it attacks. When you do, gain 1 life.").attack_condition(at_twenty)],
    ..EXERTER
};

fn one_to_attack_them(q: &dyn Query, me: &Asker, _: ObjectId, target: Target) -> Option<ManaCost> {
    (target == Target::Player(q.obj(me.source).controller)).then_some(ManaCost { generic: 1, ..ManaCost::ZERO })
}

/// "Creatures can't attack you unless their controller pays {1} for each creature they control that's attacking you."
static TOLL: CardDef = CardDef { name: "Attack Option Toll", types: TypeSet::of(&[CardType::Enchantment]), attack_costs: &[one_to_attack_them], ..CardDef::BLANK };
/// "Creatures can't attack you unless their controller pays {W/P} for each creature they control that's attacking you" (Norn's
/// Annex's cost, on the player alone).
fn white_phyrexian_to_attack_them(q: &dyn Query, me: &Asker, _: ObjectId, target: Target) -> Option<ManaCost> {
    let mut phyrexian = [0; 5];
    phyrexian[Color::White as usize] = 1;
    (target == Target::Player(q.obj(me.source).controller)).then_some(ManaCost { phyrexian, ..ManaCost::ZERO })
}

static ANNEX: CardDef = CardDef { name: "Attack Option Annex", types: TypeSet::of(&[CardType::Artifact]), attack_costs: &[white_phyrexian_to_attack_them], ..CardDef::BLANK };

/// The same with {U/P}: a cost no printed card asks, beside Norn's Annex's, for a payment of more Phyrexian symbols than one
/// color holds.
fn blue_phyrexian_to_attack_them(q: &dyn Query, me: &Asker, _: ObjectId, target: Target) -> Option<ManaCost> {
    let mut phyrexian = [0; 5];
    phyrexian[Color::Blue as usize] = 1;
    (target == Target::Player(q.obj(me.source).controller)).then_some(ManaCost { phyrexian, ..ManaCost::ZERO })
}

static BLUE_ANNEX: CardDef = CardDef { name: "Attack Option Blue Annex", types: TypeSet::of(&[CardType::Artifact]), attack_costs: &[blue_phyrexian_to_attack_them], ..CardDef::BLANK };
/// The ability enlist, to be given by an effect.
static GIFT: CardDef = CardDef { name: "Attack Option Gift", types: TypeSet::of(&[CardType::Sorcery]), delayed: &[ENLIST], ..CardDef::BLANK };

fn game() -> Game {
    let table: &'static CardTable =
        Box::leak(Box::new(CardTable::new([&FILLER, &RECRUITER, &DOUBLE, &BEAR, &PUMPER, &VIGILANT, &HASTY, &ELF, &EXERTER, &CONDITIONAL, &TOLL, &GIFT, &ANNEX, &BLUE_ANNEX])));
    let deck = || vec![table.id_of(&FILLER); 20];
    let mut g = Game::new(table, &Setup { decks: vec![deck(), deck()], ..Default::default() });
    g.verify = true;
    while !g.state.agenda.is_empty() {
        g.state.frame_remove(0);
    }
    g.state.update_turn(|t| {
        t.active = YOU;
        t.priority = YOU;
        t.number = 1;
        t.set_step(Step::PrecombatMain);
    });
    g.state.frame_push(Frame::new(FrameKind::Priority));
    g.state.update_player(YOU, |p| p.turn_began = 1);
    g
}

fn put(g: &mut Game, def: &'static CardDef, owner: PlayerId) -> ObjectId {
    let id = g.state.push_obj(Obj::new(g.cards.id_of(def), owner, Zone::BATTLEFIELD));
    let timestamp = g.state.timestamp();
    g.state.update_obj(id, |o| o.timestamp = timestamp);
    g.state.zone_push(Zone::BATTLEFIELD, id);
    g.state.see_statics();
    id
}

/// Runs the game, passing priority and putting simultaneous triggers on the stack in the order offered, until another
/// question is asked; gives it back.
fn next(g: &mut Game) -> Request {
    for _ in 0..400 {
        g.run();
        let r = g.pending().cloned().expect("a question");
        match &r.kind {
            RequestKind::Priority { .. } => g.answer(r.id, r.player, Answer::Pass).expect("passing"),
            RequestKind::OrderTriggers { sources, .. } => g.answer(r.id, r.player, Answer::Order((0..sources.len() as u32).collect())).expect("an order"),
            _ => return r,
        }
    }
    panic!("no question but priority");
}

fn answer(g: &mut Game, r: &Request, a: Answer) {
    g.answer(r.id, r.player, a).expect("a legal answer");
}

/// Declares `attackers` attacking the opponent.
fn attack(g: &mut Game, attackers: &[ObjectId]) {
    let r = next(g);
    assert!(matches!(r.kind, RequestKind::DeclareAttackers { .. }), "declare attackers, not {:?}", r.kind);
    answer(g, &r, Answer::Attack(attackers.iter().map(|&a| (a, Target::Player(THEM))).collect()));
}

/// The next question, which must be a choice of a creature to enlist for `attacker`: the creatures offered.
fn enlist_offer(g: &mut Game, attacker: ObjectId) -> (Request, Vec<ObjectId>) {
    let r = next(g);
    match &r.kind {
        RequestKind::ChooseCards { source, purpose: Purpose::Enlist, cards, .. } if *source == attacker => {
            let cards = cards.clone();
            (r, cards)
        }
        other => panic!("an enlist choice for {attacker:?}, not {other:?}"),
    }
}

/// Runs to the declare blockers step, by which every trigger of the declaration has resolved.
fn to_blockers(g: &mut Game) {
    for _ in 0..400 {
        g.run();
        if g.state.turn.step() >= Step::DeclareBlockers {
            return;
        }
        let r = g.pending().cloned().expect("a question");
        match &r.kind {
            RequestKind::Priority { .. } => g.answer(r.id, r.player, Answer::Pass).expect("passing"),
            RequestKind::OrderTriggers { sources, .. } => g.answer(r.id, r.player, Answer::Order((0..sources.len() as u32).collect())).expect("an order"),
            other => panic!("{other:?}"),
        }
    }
    panic!("no declare blockers step");
}

fn power(g: &Game, id: ObjectId) -> i32 {
    Query::chars(g, id).power.unwrap_or(0)
}

/// Enlisting taps the creature (CR 702.154a, 508.1j), the linked ability gives +X/+0 where X is its power, and "whenever
/// this creature enlists a creature" triggers (CR 702.154c).
#[test]
fn enlisting_taps_the_creature_and_adds_its_power() {
    let mut g = game();
    let recruiter = put(&mut g, &RECRUITER, YOU);
    let bear = put(&mut g, &BEAR, YOU);
    attack(&mut g, &[recruiter]);
    let (r, offered) = enlist_offer(&mut g, recruiter);
    assert_eq!(offered, vec![bear]);
    answer(&mut g, &r, Answer::Cards(vec![bear]));
    to_blockers(&mut g);
    assert!(g.state.obj(bear).tapped);
    assert_eq!(power(&g, recruiter), 5);
    assert_eq!(g.state.player(YOU).life, 21, "whenever it enlists a creature");
}

/// Runs to the declaration's triggers on the stack and the attacking player's priority, and takes each of the `taken`
/// abilities of `of` in turn, each resolving before the next: the "{0}: +2/+0" (0) and the sacrifice (1).
fn respond(g: &mut Game, of: ObjectId, taken: &[usize]) {
    for &index in taken {
        loop {
            g.run();
            let r = g.pending().cloned().expect("a question");
            match &r.kind {
                RequestKind::Priority { options } if r.player == YOU => {
                    assert!(!g.state.stack.is_empty(), "the linked ability waits on the stack");
                    let o = options.iter().find(|o| o.source == Some(of) && o.kind == ActionKind::ActivateAbility && o.index as usize == index).cloned();
                    answer(g, &r, Answer::Act(Action::of(&o.expect("the ability"))));
                    break;
                }
                RequestKind::OrderTriggers { sources, .. } => answer(g, &r, Answer::Order((0..sources.len() as u32).collect())),
                other => panic!("{other:?}"),
            }
        }
        // Both pass: what was just activated resolves, and the attacking player has priority again.
        let depth = g.state.stack.len();
        while g.state.stack.len() >= depth {
            g.run();
            let r = g.pending().cloned().expect("a question");
            assert!(matches!(r.kind, RequestKind::Priority { .. }), "{:?}", r.kind);
            answer(g, &r, Answer::Pass);
        }
    }
}

/// X is read as the linked ability resolves (CR 608.2h): the enlisted creature's power then, not as it was tapped.
#[test]
fn x_is_the_enlisted_creatures_power_as_the_ability_resolves() {
    let mut g = game();
    let recruiter = put(&mut g, &RECRUITER, YOU);
    let pumper = put(&mut g, &PUMPER, YOU);
    attack(&mut g, &[recruiter]);
    let (r, _) = enlist_offer(&mut g, recruiter);
    answer(&mut g, &r, Answer::Cards(vec![pumper]));
    respond(&mut g, pumper, &[0]);
    to_blockers(&mut g);
    assert_eq!(power(&g, pumper), 5);
    assert_eq!(power(&g, recruiter), 2 + 5);
}

/// An enlisted creature gone by then is read as it last existed on the battlefield (CR 608.2h).
#[test]
fn x_of_an_enlisted_creature_that_left_is_its_last_known_power() {
    let mut g = game();
    let recruiter = put(&mut g, &RECRUITER, YOU);
    let pumper = put(&mut g, &PUMPER, YOU);
    attack(&mut g, &[recruiter]);
    let (r, _) = enlist_offer(&mut g, recruiter);
    answer(&mut g, &r, Answer::Cards(vec![pumper]));
    respond(&mut g, pumper, &[0, 1]);
    to_blockers(&mut g);
    assert!(!g.state.battlefield().contains(&pumper));
    assert_eq!(power(&g, recruiter), 2 + 5);
}

/// Choosing no creature pays nothing: nothing is tapped and nothing triggers.
#[test]
fn enlisting_nothing_pays_nothing() {
    let mut g = game();
    let recruiter = put(&mut g, &RECRUITER, YOU);
    let bear = put(&mut g, &BEAR, YOU);
    attack(&mut g, &[recruiter]);
    let (r, _) = enlist_offer(&mut g, recruiter);
    answer(&mut g, &r, Answer::Cards(Vec::new()));
    to_blockers(&mut g);
    assert!(!g.state.obj(bear).tapped);
    assert_eq!(power(&g, recruiter), 2);
    assert_eq!(g.state.player(YOU).life, 20);
    assert!(!g.log.iter().any(|r| matches!(r.event, Event::PaidToAttack { .. })));
}

/// Only an untapped creature that isn't attacking (a vigilant attacker included) and has haste or has been controlled since
/// the turn began may be enlisted (the rulings, CR 702.154a); a creature can't enlist itself (CR 702.154c).
#[test]
fn only_a_nonattacking_untapped_creature_free_of_summoning_sickness_is_offered() {
    let mut g = game();
    let recruiter = put(&mut g, &RECRUITER, YOU);
    let bear = put(&mut g, &BEAR, YOU);
    let vigilant = put(&mut g, &VIGILANT, YOU);
    let hasty = put(&mut g, &HASTY, YOU);
    let new = put(&mut g, &BEAR, YOU);
    g.state.update_obj(new, |o| o.controlled_since = 5);
    g.state.update_player(YOU, |p| p.turn_began = 5);
    for id in [recruiter, bear, vigilant] {
        g.state.update_obj(id, |o| o.controlled_since = 1);
    }
    let tapped = put(&mut g, &BEAR, YOU);
    g.state.update_obj(tapped, |o| {
        o.tapped = true;
        o.controlled_since = 1;
    });
    let theirs = put(&mut g, &BEAR, THEM);
    attack(&mut g, &[recruiter, vigilant]);
    let (_, offered) = enlist_offer(&mut g, recruiter);
    assert!(offered.contains(&bear) && offered.contains(&hasty), "{offered:?}");
    for not in [recruiter, vigilant, new, tapped, theirs] {
        assert!(!offered.contains(&not), "{not:?} is offered");
    }
}

/// A creature can't be tapped for more than one enlist ability (the rulings): the second attacker has none left to enlist,
/// and is not asked.
#[test]
fn a_creature_is_enlisted_once() {
    let mut g = game();
    let first = put(&mut g, &RECRUITER, YOU);
    let second = put(&mut g, &RECRUITER, YOU);
    let bear = put(&mut g, &BEAR, YOU);
    attack(&mut g, &[first, second]);
    let (r, offered) = enlist_offer(&mut g, first);
    assert_eq!(offered, vec![bear]);
    answer(&mut g, &r, Answer::Cards(vec![bear]));
    to_blockers(&mut g);
    assert_eq!((power(&g, first), power(&g, second)), (5, 2));
}

/// Two instances of enlist work apart (CR 702.154d): each taps its own creature and its linked ability triggers only for its
/// own cost - two instances of one ability given twice too.
#[test]
fn each_instance_of_enlist_pays_and_triggers_on_its_own() {
    let mut g = game();
    let double = put(&mut g, &DOUBLE, YOU);
    let bear = put(&mut g, &BEAR, YOU);
    let hasty = put(&mut g, &HASTY, YOU);
    attack(&mut g, &[double]);
    let (r, offered) = enlist_offer(&mut g, double);
    assert_eq!(offered.len(), 2);
    answer(&mut g, &r, Answer::Cards(vec![bear]));
    let (r, offered) = enlist_offer(&mut g, double);
    assert_eq!(offered, vec![hasty], "the bear is enlisted already");
    answer(&mut g, &r, Answer::Cards(vec![hasty]));
    to_blockers(&mut g);
    assert_eq!(power(&g, double), 1 + 3 + 1);

    // The same ability given twice by effects: one AbilityRef, two instances.
    let mut g = game();
    let bear = put(&mut g, &BEAR, YOU);
    let other = put(&mut g, &BEAR, YOU);
    let hasty = put(&mut g, &HASTY, YOU);
    let given = AbilityRef::delayed(g.cards.id_of(&GIFT), 0);
    for _ in 0..2 {
        let timestamp = g.state.timestamp();
        g.state.effect_push(ContinuousEffect { source: bear, controller: YOU, timestamp, affected: bear, kind: EffectKind::AddAbility(given), duration: Duration::EndOfTurn, words: Words::NONE });
    }
    attack(&mut g, &[bear]);
    let (r, _) = enlist_offer(&mut g, bear);
    answer(&mut g, &r, Answer::Cards(vec![other]));
    let (r, _) = enlist_offer(&mut g, bear);
    answer(&mut g, &r, Answer::Cards(vec![hasty]));
    to_blockers(&mut g);
    assert_eq!(power(&g, bear), 3 + 3 + 1, "each instance's ability counts its own creature once");
}

/// An attacker that has lost its abilities (CR 613.1f) has no enlist to pay.
#[test]
fn an_attacker_without_its_abilities_is_offered_nothing() {
    let mut g = game();
    let recruiter = put(&mut g, &RECRUITER, YOU);
    put(&mut g, &BEAR, YOU);
    let timestamp = g.state.timestamp();
    g.state.effect_push(ContinuousEffect { source: recruiter, controller: THEM, timestamp, affected: recruiter, kind: EffectKind::LoseAllAbilities, duration: Duration::EndOfTurn, words: Words::NONE });
    attack(&mut g, &[recruiter]);
    to_blockers(&mut g);
    assert_eq!(power(&g, recruiter), 2);
}

/// The optional costs are locked in with the costs to attack and paid after the mana abilities (CR 508.1h-j): an Elf chosen
/// to be enlisted and then tapped for the {1} the attack costs can't be tapped again, so the declaration is illegal and
/// taken back (CR 508.1, 733.1), the Elf untapped and attackers declared again.
#[test]
fn a_creature_to_enlist_tapped_for_mana_meanwhile_takes_the_declaration_back() {
    let mut g = game();
    put(&mut g, &TOLL, THEM);
    let recruiter = put(&mut g, &RECRUITER, YOU);
    let elf = put(&mut g, &ELF, YOU);
    attack(&mut g, &[recruiter]);
    let (r, _) = enlist_offer(&mut g, recruiter);
    answer(&mut g, &r, Answer::Cards(vec![elf]));
    let r = next(&mut g);
    let RequestKind::PayCosts { options, .. } = &r.kind else { panic!("paying the toll, not {:?}", r.kind) };
    let tap_elf = options.iter().find(|o| o.source == Some(elf) && o.kind == ActionKind::ActivateAbility).cloned().expect("the Elf's mana ability");
    answer(&mut g, &r, Answer::Act(Action::of(&tap_elf)));
    let r = next(&mut g);
    assert!(matches!(r.kind, RequestKind::DeclareAttackers { .. }), "declared again, not {:?}", r.kind);
    assert!(!g.state.obj(elf).tapped && !g.state.obj(recruiter).tapped);
}

/// Exert (CR 701.43d): yes exerts it, so it won't untap during its controller's next untap step (CR 701.43a), and the linked
/// ability triggers; no does neither.
#[test]
fn exerting_as_it_attacks_is_a_choice_with_its_own_trigger() {
    for exert in [true, false] {
        let mut g = game();
        let exerter = put(&mut g, &EXERTER, YOU);
        attack(&mut g, &[exerter]);
        let r = next(&mut g);
        assert!(matches!(r.kind, RequestKind::May { source, purpose: Purpose::Ask(q) } if source == exerter && q.verb == "Exert"), "{:?}", r.kind);
        answer(&mut g, &r, if exert { Answer::Yes } else { Answer::No });
        to_blockers(&mut g);
        assert_eq!(g.state.obj(exerter).marks.exerted_by, if exert { 1 << YOU.0 } else { 0 });
        assert_eq!(g.state.player(YOU).life, if exert { 21 } else { 20 });
    }
}

/// A cost to attack of 128 {W/P} - four Annexes and 32 attackers - each paid with 2 life (CR 107.4f, 508.1h-j): 256 life,
/// paid in full. The life is counted 32-bit, so it never wraps to nothing and lets the attack go free.
#[test]
fn a_cost_to_attack_of_many_phyrexian_symbols_paid_with_life_takes_all_of_it() {
    let mut g = game();
    for _ in 0..4 {
        put(&mut g, &ANNEX, THEM);
    }
    let bears: Vec<ObjectId> = (0..32).map(|_| put(&mut g, &BEAR, YOU)).collect();
    g.state.update_player(YOU, |p| p.life = 300);
    attack(&mut g, &bears);
    let r = next(&mut g);
    let RequestKind::ChooseModes { modes, at_most, .. } = &r.kind else { panic!("which {{W/P}} to pay with life, not {:?}", r.kind) };
    assert_eq!((modes.len(), *at_most), (128, 128));
    answer(&mut g, &r, Answer::Modes((0..128).collect()));
    // Nothing is left to pay in mana, so nothing more is asked: the attackers are declared.
    g.run();
    assert_eq!(g.state.player(YOU).life, 300 - 256);
    assert_eq!(g.state.attackers.len(), 32);
}

/// 320 Phyrexian symbols to attack - three {W/P} and two {U/P} for each of 64 attackers - are more than a choice of modes
/// holds (255): the player is asked about them a page at a time, 255 then 65, each up to what the life left covers, and every
/// one paid with 2 life takes 640 life (CR 107.4f, 508.1h-j). None is lost to a count cut to a byte.
#[test]
fn more_phyrexian_symbols_than_a_choice_holds_are_asked_a_page_at_a_time() {
    let mut g = game();
    for _ in 0..3 {
        put(&mut g, &ANNEX, THEM);
    }
    for _ in 0..2 {
        put(&mut g, &BLUE_ANNEX, THEM);
    }
    let bears: Vec<ObjectId> = (0..64).map(|_| put(&mut g, &BEAR, YOU)).collect();
    g.state.update_player(YOU, |p| p.life = 1000);
    attack(&mut g, &bears);
    let r = next(&mut g);
    let RequestKind::ChooseModes { modes, at_most, .. } = &r.kind else { panic!("which to pay with life, not {:?}", r.kind) };
    assert_eq!((modes.len(), *at_most), (255, 255));
    answer(&mut g, &r, Answer::Modes((0..255).collect()));
    let r = next(&mut g);
    let RequestKind::ChooseModes { modes, at_most, .. } = &r.kind else { panic!("the rest, not {:?}", r.kind) };
    assert_eq!((modes.len(), *at_most), (65, 65));
    answer(&mut g, &r, Answer::Modes((0..65).collect()));
    g.run();
    assert_eq!(g.state.player(YOU).life, 1000 - 640);
    assert_eq!(g.state.attackers.len(), 64);
}

/// The second page of Phyrexian symbols is offered only as many as the life the first left covers (CR 119.4): at 520 life,
/// 255 paid with life on the first page leave 10, so 5 of the second page's 65 at most.
#[test]
fn a_later_page_of_phyrexian_symbols_offers_only_what_the_life_left_covers() {
    let mut g = game();
    for _ in 0..3 {
        put(&mut g, &ANNEX, THEM);
    }
    for _ in 0..2 {
        put(&mut g, &BLUE_ANNEX, THEM);
    }
    let bears: Vec<ObjectId> = (0..64).map(|_| put(&mut g, &BEAR, YOU)).collect();
    g.state.update_player(YOU, |p| p.life = 520);
    attack(&mut g, &bears);
    let r = next(&mut g);
    answer(&mut g, &r, Answer::Modes((0..255).collect()));
    let r = next(&mut g);
    let RequestKind::ChooseModes { modes, at_most, .. } = &r.kind else { panic!("the rest, not {:?}", r.kind) };
    assert_eq!((modes.len(), *at_most), (65, 5));
}

#[test]
fn attack_permission_is_checked_at_declaration_not_trigger_resolution() {
    for allowed in [true, false] {
        let mut g = game();
        let source = put(&mut g, &CONDITIONAL, YOU);
        if !allowed { g.state.update_player(YOU, |p| p.life = 19); }
        attack(&mut g, &[source]);
        if allowed {
            let r = next(&mut g);
            assert!(matches!(r.kind, RequestKind::May { purpose: Purpose::Ask(q), .. } if q.verb == "Exert"), "permitted exert must be offered");
            answer(&mut g, &r, Answer::Yes);
        }
        to_blockers(&mut g);
        assert_eq!(g.state.obj(source).marks.exerted_by != 0, allowed, "static permission controls payment");
        assert_eq!(g.state.player(YOU).life, if allowed { 21 } else { 19 }, "linked trigger must resolve despite changed static condition");
    }
}

#[test]
fn exert_choice_history_trigger_and_duration_survive_undo_redo_and_a_declined_branch() {
    let mut g = game();
    let source = put(&mut g, &EXERTER, YOU);
    attack(&mut g, &[source]);
    let r = next(&mut g);
    let before = g.checkpoint();
    answer(&mut g, &r, Answer::Yes);
    to_blockers(&mut g);
    let yes = g.checkpoint();
    let history = g.log.clone();
    assert!(history.iter().any(|r| matches!(r.event, Event::Exerted { permanent, player: YOU } if permanent == source)), "exert must be recorded");
    g.goto(before);
    assert_eq!(g.state.obj(source).marks.exerted_by, 0, "undo exert duration");
    assert!(!g.log.iter().any(|r| matches!(r.event, Event::Exerted { .. })), "undo exert history");
    answer(&mut g, &r, Answer::No);
    to_blockers(&mut g);
    let no = g.checkpoint();
    assert_eq!(g.state.player(YOU).life, 20, "decline branch has no trigger");
    g.goto(yes);
    assert_eq!(g.log, history, "redo exact history");
    assert_eq!(g.state.obj(source).marks.exerted_by, 1, "redo exert duration");
    assert_eq!(g.state.player(YOU).life, 21, "redo trigger effect");
    g.goto(no);
    assert_eq!(g.state.obj(source).marks.exerted_by, 0, "undo across common ancestor");
}

#[test]
fn put_into_combat_is_not_an_exert_permission_or_a_paid_attack_cost() {
    let mut g = game();
    let source = put(&mut g, &EXERTER, YOU);
    g.state.update_turn(|t| { t.set_step(Step::DeclareAttackers); t.attacked = true; });
    let mut cx = crate::Cx { g: &mut g, frame: usize::MAX, cursor: 0, mana_source: None, entering: None, lineage: Vec::new(), damage: None, asked_at: None, doer: None, producing: None, reached: None };
    crate::ops::enter_attacking(&mut cx, &[source], Target::Player(THEM));
    assert!(g.state.attackers.iter().any(|a| a.id == source), "creature entered attacking");
    assert_eq!(g.state.obj(source).marks.exerted_by, 0, "putting into combat does not exert");
    assert!(!g.log.iter().any(|r| matches!(r.event, Event::PaidToAttack { .. } | Event::Exerted { .. })), "no declaration cost event");
    assert!(g.state.triggers.is_empty(), "no linked exert trigger");
}
