//! The engine's log as the entries the client shows.
//!
//! Two stages. `Tracker::collect` reads what happened, once, over the whole game and without regard to who is
//! watching (`Proto`): every native `Event` through the one function `proto_of`, plus what only a comparison of the
//! position before and after can say (an ability put on the stack or resolved, a player lost). Then `Proto::tell`
//! says what one seat may be told of it, in that seat's handles; the same proto has the same sequence number for every
//! seat, so a later retraction can name it.
//!
//! # Mapping
//!
//! | native                                            | `GameEventView`                                     |
//! |---------------------------------------------------|-----------------------------------------------------|
//! | `StepBegan` (and the untap step, read off a wrap)  | `StepBeganEvent`                                    |
//! | `ZoneChange`                                      | `ObjectMovedEvent`, or `TokenCreatedEvent` from nowhere |
//! | `LifeChange`                                      | `LifeChangedEvent`                                  |
//! | `Damage` to an object                             | `DamageMarkedEvent`                                 |
//! | `Tapped`                                          | `TapChangedEvent`                                   |
//! | `SpellCast`                                       | `SpellCastEvent` (targets, X, modes, kicks)         |
//! | `Revealed`                                        | `CardsRevealedEvent`                                |
//! | `Looked` (those of one player in a row together)  | `CardsLookedAtEvent`, the cards known to the looker alone |
//! | `CountersPut` (every kind, by its name)           | `CountersPutEvent`                                  |
//! | `Transformed`                                     | `TransformedEvent`                                  |
//! | a new activated/triggered item on the stack       | `AbilityActivatedEvent`                             |
//! | an ability item gone from the stack               | `AbilityResolvedEvent`                              |
//! | a player's `lost`, the outcome                    | `PlayerLostEvent`, `GameEndedEvent`                 |
//! | monarch, extra turns                              | `MonarchBecameEvent`, `ExtraTurnAddedEvent`         |
//! | `DieRolled` (the result, modifiers applied)        | `DieRolledEvent`                                    |
//! | `CoinFlipped`                                     | `CoinFlippedEvent`                                  |
//! | `TurnEnded` (an effect ended the turn, CR 724.1)   | `TurnEndedEvent`                                    |
//! | `Scried`, `Surveilled` (CR 701.22, 701.25)        | `ScriedEvent`, `SurveilledEvent` (the number)       |
//! | the answers to the opening's questions            | `StartingPlayerChosenEvent`, `MulliganTakenEvent`, `HandKeptEvent` |
//!
//! Not told, because the engine has no event or state for it: untaps, shuffles (`LibraryShuffledEvent`), where a scry
//! put the cards (`LibraryArrangedEvent`; a surveil's arrive as moves), cycling and discard-as-cost as such (they arrive as moves), extra phases,
//! day and night, a random player chosen (`ChosenAtRandom` is a bare index; the effect that chose says what), fizzling (`SpellFizzledEvent`), retractions (a cast given up is never
//! published), counters other than lore, damage to players as such (life covers it), mana abilities, combat
//! declarations as log lines (combat is in the view).

use std::collections::{HashMap, HashSet};

use cantrip_engine::*;

use crate::handles::HandleSpace;
use crate::protocol::*;
use crate::view::*;

/// What happened, for nobody in particular.
#[derive(Clone, Debug)]
pub enum Proto {
    Step { turn: i32, step: Step, active: PlayerId },
    Moved { old: ObjectId, new: ObjectId, from: Zone, to: Zone, reason: ZoneReason },
    Created { object: ObjectId, controller: PlayerId, name: String },
    Life { player: PlayerId, delta: i32, total: i32 },
    Damage { object: ObjectId, amount: i32 },
    Tap { object: ObjectId, tapped: bool },
    Cast { spell: ObjectId, controller: PlayerId, targets: Vec<Target>, x: Option<i32>, modes: Vec<String>, kicked: i32, gift: Option<PlayerId> },
    AbilityPut { serial: u32, source: ObjectId, controller: PlayerId, text: String, targets: Vec<Target>, triggered: bool, modes: Vec<String> },
    /// `first` is the log sequence of the first entry its effect made: the entries from there up to this one are what it
    /// did, and none before.
    AbilityResolved { serial: u32, source: ObjectId, controller: PlayerId, targets: Vec<Target>, first: u64 },
    Revealed { player: PlayerId, cards: Vec<ObjectId> },
    /// Hidden cards one player looked at: known to that player, and to nobody else.
    Looked { player: PlayerId, cards: Vec<ObjectId> },
    Counters { object: ObjectId, name: String, count: i32 },
    Transformed { object: ObjectId, from: String, to: String },
    Lost { player: PlayerId, reason: LossReasonView },
    Ended { outcome: GameOutcomeView, winning_team: i32, draw_reason: DrawReasonView },
    Monarch { player: PlayerId },
    ExtraTurn { player: PlayerId, count: i32 },
    /// Entries told earlier that no longer stand: a mana ability taken back, and with it what it was told to have done.
    Retracted { sequences: Vec<u64> },
    StartingPlayer { chooser: PlayerId, player: PlayerId },
    Mulligan { player: PlayerId, count: i32 },
    Kept { player: PlayerId, cards: i32 },
    Die { player: PlayerId, sides: i32, result: i32 },
    Coin { player: PlayerId, result: CoinFlip },
    /// An effect ended the turn of `player` (CR 724.1).
    TurnEnded { player: PlayerId },
    /// `player` scried `count` (CR 701.22).
    Scried { player: PlayerId, count: i32 },
    /// `player` surveilled `count` (CR 701.25).
    Surveilled { player: PlayerId, count: i32 },
}

/// The ability a stack item was put on the stack as, kept until it leaves.
#[derive(Clone, Debug)]
struct Seen {
    source: ObjectId,
    ability: AbilityRef,
    controller: PlayerId,
    targets: Vec<Target>,
}

/// What has been told so far, to say what is new.
#[derive(Clone, Default)]
pub struct Tracker {
    turn: i32,
    last_step: Option<Step>,
    /// The turn and its player of the last step told.
    last_turn: (i32, PlayerId),
    active: PlayerId,
    life: Vec<i32>,
    stack: HashMap<u32, Seen>,
    lost: Vec<bool>,
    ended: bool,
    monarch: Option<PlayerId>,
    extra_turns: usize,
    /// What made the latest log record, and the sequence the first entry it made was or will be given: a resolution
    /// that stopped to ask a question carries on in a later collection.
    run: Option<(Cause, u64)>,
    /// Why the game was drawn, once the history has said (CR 104.4).
    draw_reason: DrawReasonView,
}

fn spell_facts(game: &Game, spell: ObjectId) -> (Vec<Target>, Option<i32>, Vec<String>, i32, Option<PlayerId>) {
    let item = game.state.stack.iter().rev().find(|i| i.kind == StackKind::Spell && i.object == spell);
    let Some(item) = item else { return (Vec::new(), None, Vec::new(), 0, None) };
    let def = game.cards.get(game.chars(spell).card);
    let mut modes = Vec::new();
    if let Some(modal) = def.modal {
        for (i, mode) in modal.modes.iter().enumerate() {
            if modal.times(item.modes, i) > 0 {
                modes.push(mode.text.to_string());
            }
        }
    }
    let x = (def.cost.x > 0).then_some(i32::from(item.x));
    // The log says "kicked" for this count, so it is the kicker's alone: an optional additional cost (`paid`: behold, blight,
    // bargain) is not a kicker, and its payment shows in the log as what it did (a card revealed, a creature sacrificed).
    (game.state.targets(item).to_vec(), x, modes, i32::from(item.kicked), item.gift)
}

impl Tracker {
    pub fn new(game: &Game) -> Tracker {
        Tracker {
            turn: game.state.turn.number as i32,
            last_step: None,
            active: game.state.turn.active,
            life: game.state.players.iter().map(|p| p.life).collect(),
            lost: vec![false; game.state.players.len()],
            ..Default::default()
        }
    }

    /// The one place a native event becomes something to tell. Everything else a seat hears is a comparison of
    /// positions (`collect`).
    fn proto_of(&mut self, game: &Game, record: &Record, out: &mut Vec<Proto>) {
        match record.event {
            Event::StepBegan { step } => {
                // A new turn, which begins with its untap step (no step event of its own): one whose number or player is not
                // the last step's. A step begun again in the same turn is not one - a second combat damage step (CR 510.4),
                // an additional combat phase (CR 500.8), another cleanup step (CR 514.3a, 724.1d).
                let turn = (record.turn as i32, record.active);
                let wrapped = self.last_step.is_none() || self.last_turn != turn;
                if wrapped {
                    (self.turn, self.active) = turn;
                    out.push(Proto::Step { turn: self.turn, step: Step::Untap, active: self.active });
                }
                self.last_step = Some(step);
                self.last_turn = turn;
                out.push(Proto::Step { turn: turn.0, step, active: turn.1 });
            }
            Event::ZoneChange { old, new, from, to, reason } => {
                if to.is(ZoneKind::Gone) {
                    return;
                }
                if from.is(ZoneKind::Gone) {
                    let name = game.chars(new).name.to_string();
                    out.push(Proto::Created { object: new, controller: game.state.obj(new).controller, name });
                } else {
                    out.push(Proto::Moved { old, new, from, to, reason });
                }
            }
            Event::LifeChange { player, delta } => {
                let total = self.life.get_mut(player.index()).map(|l| {
                    *l += delta;
                    *l
                });
                out.push(Proto::Life { player, delta, total: total.unwrap_or(game.state.player(player).life) });
            }
            Event::Damage { target: Target::Object(object), amount, .. } => {
                out.push(Proto::Damage { object, amount: i32::from(amount) });
            }
            Event::Tapped { permanent } => out.push(Proto::Tap { object: permanent, tapped: true }),
            Event::SpellCast { spell, player, .. } => {
                let (targets, x, modes, kicked, gift) = spell_facts(game, spell);
                out.push(Proto::Cast { spell, controller: player, targets, x, modes, kicked, gift });
            }
            Event::Revealed { player, card } => out.push(Proto::Revealed { player, cards: vec![card] }),
            Event::Looked { player, card } => match out.last_mut() {
                Some(Proto::Looked { player: looker, cards }) if *looker == player => cards.push(card),
                _ => out.push(Proto::Looked { player, cards: vec![card] }),
            },
            Event::CountersPut { object, kind, amount, .. } => {
                out.push(Proto::Counters { object, name: kind.name().to_string(), count: i32::from(amount) });
            }
            Event::Transformed { object } => {
                let def = game.cards.get(game.state.obj(object).card);
                let (front, back) = (def.name.to_string(), def.back.map_or(String::new(), |b| b.name.to_string()));
                let up = game.state.obj(object).transformed;
                let (from, to) = if up { (front, back) } else { (back, front) };
                out.push(Proto::Transformed { object, from, to });
            }
            Event::DieRolled { player, sides, result, .. } => {
                out.push(Proto::Die { player, sides: i32::from(sides), result: i32::from(result) });
            }
            Event::CoinFlipped { player, result } => out.push(Proto::Coin { player, result }),
            Event::TurnEnded { player } => out.push(Proto::TurnEnded { player }),
            // A loss the rules recorded says why (CR 104.3): Phyrexian Unlife's player at -2 life loses to poison, not life.
            Event::GameLost { player, reason } if !self.lost.get(player.index()).copied().unwrap_or(true) => {
                self.lost[player.index()] = true;
                let reason = match reason {
                    LossReason::Life => LossReasonView::Life,
                    LossReason::EmptyLibrary => LossReasonView::EmptyLibrary,
                    LossReason::Poison => LossReasonView::Poison,
                    LossReason::Effect => LossReasonView::Effect,
                    LossReason::Conceded => LossReasonView::Conceded,
                };
                out.push(Proto::Lost { player, reason });
            }
            Event::GameDrawn { reason } => {
                self.draw_reason = match reason {
                    DrawReason::AllLost => DrawReasonView::AllLost,
                    DrawReason::MandatoryLoop => DrawReasonView::MandatoryLoop,
                    DrawReason::EventLimit => DrawReasonView::EventLimit,
                };
            }
            Event::Scried { player, count, .. } => out.push(Proto::Scried { player, count: crate::prompt::wire(count) }),
            Event::Surveilled { player, count, .. } => out.push(Proto::Surveilled { player, count: crate::prompt::wire(count) }),
            _ => {}
        }
    }

    /// Everything the game did since the last collection: the log entries in `records`, then what the position says
    /// that they do not. `next` is the sequence the first entry returned will be given.
    pub fn collect(&mut self, game: &Game, records: &[Record], next: u64) -> Vec<Proto> {
        let state = &game.state;
        let now: HashSet<u32> = state.stack.iter().filter(|i| i.kind != StackKind::Spell).map(|i| i.serial).collect();

        let mut out = Vec::new();
        for record in records {
            if self.run.is_none_or(|(cause, _)| cause != record.cause) {
                self.run = Some((record.cause, next + out.len() as u64));
            }
            // An ability that has resolved is told of where it did, after the entries its effect made (CR 608.2m).
            if let Event::Resolved { source, ability } = record.event {
                let resolved = self
                    .stack
                    .iter()
                    .filter(|(serial, seen)| seen.source == source && seen.ability == ability && !now.contains(serial))
                    .map(|(serial, _)| *serial)
                    .min();
                if let Some(seen) = resolved.and_then(|serial| self.stack.remove(&serial).map(|seen| (serial, seen))) {
                    let (serial, seen) = seen;
                    let first = match self.run {
                        Some((Cause::Ability { source: s, ability: a }, first)) if s == source && a == ability => first,
                        _ => next + out.len() as u64,
                    };
                    out.push(Proto::AbilityResolved { serial, source: seen.source, controller: seen.controller, targets: seen.targets, first });
                }
                continue;
            }
            self.proto_of(game, record, &mut out);
        }
        self.turn = state.turn.number as i32;
        self.active = state.turn.active;

        // Abilities on the stack: new ones were put there, missing ones left it without resolving - countered, or with
        // every target gone - and did nothing.
        for item in state.stack.iter().filter(|i| i.kind != StackKind::Spell) {
            if !self.stack.contains_key(&item.serial) {
                let text = game.cards.ability(item.ability).text().to_string();
                self.stack.insert(item.serial, Seen { source: item.object, ability: item.ability, controller: item.controller, targets: state.targets(item).to_vec() });
                out.push(Proto::AbilityPut {
                    serial: item.serial,
                    source: item.object,
                    controller: item.controller,
                    text,
                    targets: state.targets(item).to_vec(),
                    triggered: item.kind == StackKind::Triggered,
                    modes: Vec::new(),
                });
            }
        }
        let mut gone: Vec<u32> = self.stack.keys().copied().filter(|s| !now.contains(s)).collect();
        gone.sort();
        for serial in gone {
            if let Some(seen) = self.stack.remove(&serial) {
                let first = next + out.len() as u64;
                out.push(Proto::AbilityResolved { serial, source: seen.source, controller: seen.controller, targets: seen.targets, first });
            }
        }

        for p in state.seats() {
            let player = state.player(p);
            if player.lost && !self.lost[p.index()] {
                self.lost[p.index()] = true;
                let reason = if player.life <= 0 {
                    LossReasonView::Life
                } else if player.poison() >= 10 {
                    LossReasonView::Poison
                } else if player.drew_from_empty {
                    LossReasonView::EmptyLibrary
                } else {
                    LossReasonView::Effect
                };
                out.push(Proto::Lost { player: p, reason });
            }
            if player.monarch && self.monarch != Some(p) {
                self.monarch = Some(p);
                out.push(Proto::Monarch { player: p });
            }
        }
        if state.extra_turns.len() > self.extra_turns {
            for t in &state.extra_turns[self.extra_turns..] {
                out.push(Proto::ExtraTurn { player: t.player, count: 1 });
            }
        }
        self.extra_turns = state.extra_turns.len();

        if state.is_over() && !self.ended {
            self.ended = true;
            out.push(match state.outcome {
                Outcome::Won(p) => Proto::Ended { outcome: GameOutcomeView::TeamWon, winning_team: seat(p), draw_reason: DrawReasonView::None },
                _ => Proto::Ended { outcome: GameOutcomeView::Draw, winning_team: 0, draw_reason: self.draw_reason },
            });
        }
        out
    }

    /// A seat conceded: the host marks the loss, and says so.
    pub fn conceded(&mut self, player: PlayerId) -> Vec<Proto> {
        if let Some(l) = self.lost.get_mut(player.index()) {
            *l = true;
        }
        vec![Proto::Lost { player, reason: LossReasonView::Conceded }]
    }
}

fn reason_view(reason: ZoneReason, from: Zone, to: Zone) -> ZoneChangeReasonView {
    use ZoneChangeReasonView as V;
    match reason {
        ZoneReason::Draw => V::Draw,
        ZoneReason::Discard => V::Discard,
        ZoneReason::Mill => V::Milled,
        ZoneReason::Sacrifice => V::Sacrificed,
        ZoneReason::Destroy | ZoneReason::DestroyNoRegen => V::Destroyed,
        ZoneReason::PlayLand => V::Play,
        ZoneReason::Cast => V::Cast,
        ZoneReason::Resolve => V::Resolve,
        ZoneReason::Counter => V::Countered,
        ZoneReason::StateBased => V::StateBasedAction,
        ZoneReason::Exile => V::Exiled,
        ZoneReason::Created => V::Created,
        ZoneReason::Search => V::Searched,
        ZoneReason::Other => match (from.kind(), to.kind()) {
            (ZoneKind::Stack, _) => V::Resolve,
            (_, ZoneKind::Hand) => V::PutIntoHand,
            (_, ZoneKind::Library) => V::PutOnLibrary,
            (_, ZoneKind::Exile) => V::Exiled,
            (ZoneKind::Battlefield, _) => V::StateBasedAction,
            _ => V::Returned,
        },
    }
}

/// What one seat is working with while it is told a batch.
pub struct Teller<'a> {
    pub game: &'a Game,
    pub viewer: PlayerId,
    pub handles: &'a mut HandleSpace,
    /// Objects shown to this seat right now (a reveal, a prompt).
    pub extra: &'a HashSet<ObjectId>,
    /// Every object an event named: their handles outlive this projection by one cycle.
    pub mentioned: HashSet<ObjectId>,
}

impl Teller<'_> {
    fn visible(&self, id: ObjectId) -> bool {
        let s = &self.game.state;
        if is_ability_key(id) {
            return true;
        }
        if self.extra.contains(&id) {
            return true;
        }
        id.index() < s.objects.len() && live(s, id) && looks_into(s, self.viewer, s.obj(id).zone)
    }

    /// Minted for what the viewer can observe now, looked up for what they cannot.
    fn handle_for(&mut self, id: ObjectId) -> ViewHandle {
        self.mentioned.insert(id);
        if self.visible(id) { self.handles.handle(id) } else { self.handles.existing(id) }
    }

    fn target(&mut self, t: Target) -> Option<TargetMessage> {
        match t {
            Target::Player(p) => Some(TargetMessage::player(seat(p))),
            Target::Object(id) => {
                let h = self.handle_for(id);
                (!h.is_none()).then(|| TargetMessage::object(h))
            }
            Target::Ability(serial) => Some(TargetMessage::object(self.handles.handle(ability_key(serial)))),
            Target::None => None,
        }
    }

    pub fn tell(&mut self, proto: &Proto) -> Option<GameEventView> {
        match proto {
            Proto::Step { turn, step, active } => {
                Some(GameEventView::StepBeganEvent(StepBeganEvent { turn: *turn, step: step_view(*step), active_player: seat(*active) }))
            }
            Proto::Moved { old, new, from, to, reason } => {
                let owner = self.game.state.obj(*new).owner;
                let to_seen = looks_into(&self.game.state, self.viewer, Zone { player: if matches!(to.kind(), ZoneKind::Hand) { owner } else { to.player }, ..*to });
                let existing = self.handles.existing(*old);
                self.mentioned.insert(*old);
                self.mentioned.insert(*new);
                // A card turned face down as it moves (cast by morph or disguise, CR 708.4; manifested or cloaked from a hand,
                // CR 701.40a, 701.58a) is a new object no seat that may not look at it can tell from the others it could
                // have been (CR 708.5, 708.6): a handle it held for the card is not carried over.
                let hidden = !knows_face(&self.game.state, self.viewer, *new);
                let object = if to_seen || self.extra.contains(new) {
                    if existing.is_none() || hidden {
                        self.handles.handle(*new)
                    } else {
                        self.handles.rebind(*old, *new);
                        existing
                    }
                } else {
                    existing
                };
                Some(GameEventView::ObjectMovedEvent(ObjectMovedEvent {
                    object,
                    from: zone_kind_view(from.kind()),
                    from_owner: zone_owner(*from),
                    to: zone_kind_view(to.kind()),
                    to_owner: zone_owner(*to),
                    reason: reason_view(*reason, *from, *to),
                }))
            }
            Proto::Created { object, controller, name } => {
                let h = self.handle_for(*object);
                Some(GameEventView::TokenCreatedEvent(TokenCreatedEvent { object: h, controller: seat(*controller), name: name.clone() }))
            }
            Proto::Life { player, delta, total } => {
                Some(GameEventView::LifeChangedEvent(LifeChangedEvent { player: seat(*player), delta: *delta, new_total: *total }))
            }
            Proto::Damage { object, amount } => {
                let h = self.handle_for(*object);
                (!h.is_none()).then_some(GameEventView::DamageMarkedEvent(DamageMarkedEvent { object: h, amount: *amount }))
            }
            Proto::Tap { object, tapped } => {
                let h = self.handle_for(*object);
                (!h.is_none()).then_some(GameEventView::TapChangedEvent(TapChangedEvent { object: h, tapped: *tapped }))
            }
            Proto::Cast { spell, controller, targets, x, modes, kicked, gift } => {
                let spell = self.handle_for(*spell);
                let targets = targets.iter().filter_map(|t| self.target(*t)).collect();
                Some(GameEventView::SpellCastEvent(SpellCastEvent {
                    spell,
                    controller: seat(*controller),
                    targets,
                    x: *x,
                    modes: modes.clone(),
                    kicked: *kicked,
                    buyback: false,
                    gift_promised_to: gift.map(seat),
                }))
            }
            Proto::AbilityPut { serial, source, controller, text, targets, triggered, modes } => {
                let ability = self.handles.handle(ability_key(*serial));
                let source = self.handle_for(*source);
                let targets = targets.iter().filter_map(|t| self.target(*t)).collect();
                Some(GameEventView::AbilityActivatedEvent(AbilityActivatedEvent {
                    ability,
                    source,
                    controller: seat(*controller),
                    text: text.clone(),
                    targets,
                    triggered: *triggered,
                    modes: modes.clone(),
                }))
            }
            Proto::AbilityResolved { serial, source, controller, targets, first } => {
                let ability = self.handles.existing(ability_key(*serial));
                let source = self.handle_for(*source);
                let targets = targets.iter().filter_map(|t| self.target(*t)).collect();
                Some(GameEventView::AbilityResolvedEvent(AbilityResolvedEvent {
                    ability,
                    source,
                    controller: seat(*controller),
                    first: *first as i64,
                    targets,
                    power_toughness: Vec::new(),
                }))
            }
            Proto::Revealed { player, cards } => {
                let cards = cards.iter().map(|c| self.handle_for(*c)).collect();
                Some(GameEventView::CardsRevealedEvent(CardsRevealedEvent { player: seat(*player), cards }))
            }
            Proto::Looked { player, cards } => {
                let cards = cards.iter().map(|c| self.handle_for(*c)).collect();
                Some(GameEventView::CardsLookedAtEvent(CardsLookedAtEvent { player: seat(*player), cards }))
            }
            Proto::Counters { object, name, count } => {
                let h = self.handle_for(*object);
                (!h.is_none()).then(|| GameEventView::CountersPutEvent(CountersPutEvent { object: h, name: name.clone(), count: *count }))
            }
            Proto::Transformed { object, from, to } => {
                let h = self.handle_for(*object);
                (!h.is_none()).then(|| GameEventView::TransformedEvent(TransformedEvent { object: h, from: from.clone(), to: to.clone() }))
            }
            Proto::Lost { player, reason } => Some(GameEventView::PlayerLostEvent(PlayerLostEvent { player: seat(*player), reason: *reason })),
            Proto::Ended { outcome, winning_team, draw_reason } => Some(GameEventView::GameEndedEvent(GameEndedEvent {
                outcome: *outcome,
                winning_team: *winning_team,
                draw_reason: *draw_reason,
            })),
            Proto::Retracted { sequences } => Some(GameEventView::EventsRetractedEvent(EventsRetractedEvent {
                sequences: sequences.iter().map(|&s| s as i64).collect(),
            })),
            Proto::Monarch { player } => Some(GameEventView::MonarchBecameEvent(MonarchBecameEvent { player: seat(*player) })),
            Proto::ExtraTurn { player, count } => {
                Some(GameEventView::ExtraTurnAddedEvent(ExtraTurnAddedEvent { player: seat(*player), count: *count }))
            }
            Proto::StartingPlayer { chooser, player } => {
                Some(GameEventView::StartingPlayerChosenEvent(StartingPlayerChosenEvent { chooser: seat(*chooser), player: seat(*player) }))
            }
            Proto::Mulligan { player, count } => {
                Some(GameEventView::MulliganTakenEvent(MulliganTakenEvent { player: seat(*player), count: *count }))
            }
            Proto::Die { player, sides, result } => {
                Some(GameEventView::DieRolledEvent(DieRolledEvent { player: seat(*player), sides: *sides, result: *result }))
            }
            Proto::Coin { player, result } => Some(GameEventView::CoinFlippedEvent(CoinFlippedEvent {
                player: seat(*player),
                won: *result == CoinFlip::Won,
                called: matches!(result, CoinFlip::Won | CoinFlip::Lost),
                heads: *result == CoinFlip::Heads,
            })),
            Proto::Kept { player, cards } => Some(GameEventView::HandKeptEvent(HandKeptEvent { player: seat(*player), cards: *cards })),
            Proto::TurnEnded { player } => Some(GameEventView::TurnEndedEvent(TurnEndedEvent { player: seat(*player) })),
            Proto::Scried { player, count } => Some(GameEventView::ScriedEvent(ScriedEvent { player: seat(*player), count: *count })),
            Proto::Surveilled { player, count } => {
                Some(GameEventView::SurveilledEvent(SurveilledEvent { player: seat(*player), count: *count }))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn began(serial: u32, turn: u32, active: PlayerId, step: Step) -> Record {
        Record { serial, turn, step, active, cause: Cause::Turn, event: Event::StepBegan { step } }
    }

    /// A step begun again within one turn is no new turn: another cleanup step (CR 514.3a, 724.1d), a second combat
    /// damage step (CR 510.4). Only a step of another turn starts one, with its untap step.
    #[test]
    fn a_step_begun_again_in_the_same_turn_is_not_a_new_turn() {
        let deck = vec!["Forest".to_string(); 20];
        let game = deal(&[deck.clone(), deck], 1, false, None).expect("a game");
        let (p1, p2) = (PlayerId(0), PlayerId(1));
        let mut tracker = Tracker::new(&game);
        let records = [
            began(0, 3, p1, Step::CombatDamage),
            began(1, 3, p1, Step::CombatDamage),
            began(2, 3, p1, Step::Cleanup),
            began(3, 3, p1, Step::Cleanup),
            began(4, 4, p2, Step::Upkeep),
        ];
        let steps: Vec<(i32, Step)> = tracker
            .collect(&game, &records, 0)
            .into_iter()
            .filter_map(|p| match p {
                Proto::Step { turn, step, .. } => Some((turn, step)),
                _ => None,
            })
            .collect();
        assert_eq!(
            steps,
            vec![
                (3, Step::Untap),
                (3, Step::CombatDamage),
                (3, Step::CombatDamage),
                (3, Step::Cleanup),
                (3, Step::Cleanup),
                (4, Step::Untap),
                (4, Step::Upkeep),
            ]
        );
    }
}
