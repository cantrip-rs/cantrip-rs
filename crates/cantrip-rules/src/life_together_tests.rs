//! One instruction's life losses and simultaneous damage results (CR 101.4, 119.3, 120.4, 603.2c, 608.2h), over
//! test-only definitions of printed abilities. Gul Dukat and Archfiend of Despair supply different amounts;
//! Exsanguinate supplies one amount and uses the total; Pyrohemia and combat share the damage-result pipeline.

use crate::{Cx, Game, Setup};
use cantrip_api::*;
use cantrip_state::{Frame, FrameKind};

const YOU: PlayerId = PlayerId(0);
const FIRST: PlayerId = PlayerId(1);
const ACTIVE: PlayerId = PlayerId(2);
const LAST: PlayerId = PlayerId(3);

fn nothing(_: &mut dyn cantrip_api::Game, _: &StackItem) -> Result<(), Suspend> {
    Ok(())
}

fn opponent_lost_life(_: &dyn Query, me: &Asker, event: &Event) -> Option<Fired> {
    match *event {
        Event::LifeChange { player, delta } if player != me.controller && delta < 0 => Fired::about(Target::Player(player)),
        _ => None,
    }
}

fn opponent_lost_one(q: &dyn Query, me: &Asker, event: &Event) -> Option<Fired> {
    match *event {
        Event::LifeChange { delta: -1, .. } => opponent_lost_life(q, me, event),
        _ => None,
    }
}

static WATCHER: CardDef = CardDef {
    name: "Life Together Watcher",
    types: TypeSet::of(&[CardType::Enchantment]),
    abilities: &[
        triggered(when_asked(EventMask::LIFE_CHANGE, opponent_lost_one), nothing,
            "Whenever one or more opponents each lose exactly 1 life, put a +1/+1 counter on Ob Nixilis. Exile the top card of your library. Until your next end step, you may play that card.").once_per_batch(),
        triggered(when_asked(EventMask::LIFE_CHANGE, opponent_lost_life), nothing,
            "Whenever an opponent loses life, that player mills that many cards."),
    ],
    ..CardDef::BLANK
};

/// Bloodletter of Aclazotz's replacement, with assertions that all replacements see the original instruction's
/// pre-loss state and history, even for a player later in APNAP order.
fn opponent_loses_on_your_turn(q: &dyn Query, me: &Asker, proposal: &Proposal) -> bool {
    matches!(proposal, Proposal::LifeChange(l) if l.amount < 0 && l.player != me.controller && q.state().turn.active == me.controller)
}

fn twice(q: &dyn Query, _: &Asker, proposal: &mut Proposal) {
    assert!(q.state().players.iter().all(|p| p.life == 20), "no original life update before all replacements");
    assert!(!q.history().records.iter().any(|r| matches!(r.event, Event::LifeChange { .. })), "no original event before all replacements");
    if let Proposal::LifeChange(l) = proposal {
        l.amount *= 2;
    }
}

static DOUBLER: CardDef = CardDef {
    name: "Life Together Doubler",
    types: TypeSet::of(&[CardType::Enchantment]),
    replacements: &[replacement(ReplacementKind::LifeChange, opponent_loses_on_your_turn, twice,
        "If an opponent would lose life during your turn, they lose twice that much life instead.")],
    ..CardDef::BLANK
};

fn own_total(_: &dyn Query, me: &Asker, player: PlayerId) -> bool {
    player == me.controller
}

/// Platinum Emperion's prohibition.
static EMPERION: CardDef = CardDef {
    name: "Life Together Fixed Total",
    types: TypeSet::of(&[CardType::Enchantment]),
    cant_gain_life: &[own_total],
    cant_lose_life: &[own_total],
    ..CardDef::BLANK
};

fn opponent_gains(_: &dyn Query, me: &Asker, proposal: &Proposal) -> bool {
    matches!(proposal, Proposal::LifeChange(l) if l.amount > 0 && l.player != me.controller)
}

fn gain_to_loss(_: &dyn Query, _: &Asker, proposal: &mut Proposal) {
    if let Proposal::LifeChange(l) = proposal {
        l.amount = -l.amount;
    }
}

/// Tainted Remedy's real sign-changing replacement, for the single-player pipeline regression.
static REMEDY: CardDef = CardDef {
    name: "Life Together Remedy",
    types: TypeSet::of(&[CardType::Enchantment]),
    replacements: &[replacement(ReplacementKind::LifeChange, opponent_gains, gain_to_loss,
        "If an opponent would gain life, that player loses that much life instead.")],
    ..CardDef::BLANK
};

/// Exsanguinate at X=1: the return value is used by the next instruction, after every opponent's loss.
fn drain(g: &mut dyn cantrip_api::Game, item: &StackItem) -> Result<(), Suspend> {
    let lost = g.each_opponent_loses_life(item.controller, 1);
    g.gain_life(item.controller, lost);
    Ok(())
}

static DRAIN: CardDef = CardDef {
    name: "Life Together Drain",
    types: TypeSet::of(&[CardType::Sorcery]),
    spell: spell(&[], drain),
    ..CardDef::BLANK
};

/// Pyrohemia's activated ability, including creatures: the four-player position is outside the two-seat script harness.
fn pyrohemia(g: &mut dyn cantrip_api::Game, item: &StackItem) -> Result<(), Suspend> {
    let mut hits: Vec<_> = g.state().apnap().map(|p| (item.object, Target::Player(p), 1)).collect();
    hits.extend(g.state().battlefield().iter().copied().filter(|&id| g.chars(id).is(CardType::Creature))
        .map(|id| (item.object, Target::Object(id), 1)));
    g.damage_together(&hits);
    Ok(())
}

static PYROHEMIA: CardDef = CardDef {
    name: "Damage Life Pyrohemia",
    types: TypeSet::of(&[CardType::Enchantment]),
    colors: Some(ColorSet::of(Color::Red)),
    abilities: &[ability(cost("{R}"), pyrohemia, "{R}: Pyrohemia deals 1 damage to each creature and each player.")],
    ..CardDef::BLANK
};

static LIFELINKER: CardDef = CardDef {
    name: "Damage Life Vampire Nighthawk",
    types: TypeSet::of(&[CardType::Creature]),
    power: Some(2), toughness: Some(3),
    keywords: Keywords::of(&[Keyword::Flying, Keyword::Deathtouch, Keyword::Lifelink]),
    ..CardDef::BLANK
};

static HAWK: CardDef = CardDef {
    name: "Damage Life Healer's Hawk",
    power: Some(1), toughness: Some(1),
    keywords: Keywords::of(&[Keyword::Flying, Keyword::Lifelink]),
    ..LIFELINKER
};

static INFECT: CardDef = CardDef {
    name: "Damage Life Blighted Agent",
    power: Some(1), toughness: Some(1),
    keywords: Keywords::of(&[Keyword::Infect, Keyword::CantBeBlocked]),
    ..LIFELINKER
};

static TOXIC: CardDef = CardDef {
    name: "Damage Life Bilious Skulldweller",
    power: Some(1), toughness: Some(1),
    keywords: Keywords::of(&[Keyword::Deathtouch]),
    toxic: 1,
    ..LIFELINKER
};

fn to_you(_: &dyn Query, me: &Asker, damage: &DamageEvent) -> bool {
    damage.target == Target::Player(me.controller)
}

fn one_damage(_: &dyn Query, _: &Asker, _: &DamageEvent) -> u32 {
    1
}

static ARMOR: CardDef = CardDef {
    name: "Damage Life Urza's Armor",
    types: TypeSet::of(&[CardType::Artifact]),
    preventions: &[PreventionDef { hint: "prevent 1 damage", applies: to_you, amount: PreventionAmount::Specific(one_damage) }],
    ..CardDef::BLANK
};

fn gained(_: &dyn Query, me: &Asker, event: &Event) -> Option<Fired> {
    match *event {
        Event::LifeChange { player, delta } if player == me.controller && delta > 0 => Fired::PLAIN,
        _ => None,
    }
}

static PRIDEMATE: CardDef = CardDef {
    name: "Damage Life Ajani's Pridemate",
    types: TypeSet::of(&[CardType::Creature]),
    power: Some(2), toughness: Some(2),
    abilities: &[triggered(when_asked(EventMask::LIFE_CHANGE, gained), nothing,
        "Whenever you gain life, put a +1/+1 counter on Ajani's Pridemate.")],
    ..CardDef::BLANK
};

fn own_gain(_: &dyn Query, me: &Asker, proposal: &Proposal) -> bool {
    matches!(proposal, Proposal::LifeChange(l) if l.player == me.controller && l.amount > 0)
}

fn plus_one(q: &dyn Query, _: &Asker, proposal: &mut Proposal) {
    assert!(q.state().players.iter().all(|p| p.life == 20), "lifelink replacements precede every original life update");
    assert!(!q.history().records.iter().any(|r| matches!(r.event, Event::LifeChange { .. })), "no original life announcement yet");
    if let Proposal::LifeChange(l) = proposal {
        l.amount += 1;
    }
}

static ANGEL: CardDef = CardDef {
    name: "Damage Life Angel of Vitality",
    types: TypeSet::of(&[CardType::Enchantment]),
    replacements: &[replacement(ReplacementKind::LifeChange, own_gain, plus_one,
        "If you would gain life, you gain that much life plus 1 instead.")],
    ..CardDef::BLANK
};

/// Darien triggers on the dealt damage before any life result or result replacement (CR 120.4b-c). Its question about
/// the past excludes the whole instruction, the same boundary its result triggers use. Used in the Pyrohemia position.
fn dealt_to_you(q: &dyn Query, me: &Asker, event: &Event) -> Option<Fired> {
    if matches!(event, Event::Damage { target: Target::Player(p), .. } if *p == me.controller) {
        assert!(q.state().players.iter().all(|p| p.life == 20), "damage triggers before life results");
        assert_eq!(q.state().obj(me.source).damage, 0, "marks are also results, not the triggering damage");
        assert!(!q.history().records.iter().any(|r| matches!(r.event, Event::LifeChange { .. } | Event::Damage { .. })),
            "life results and damage have the same pre-event history");
        Fired::PLAIN
    } else {
        None
    }
}

static DARIEN: CardDef = CardDef {
    name: "Damage Life Darien, King of Kjeldor",
    types: TypeSet::of(&[CardType::Creature]),
    power: Some(3), toughness: Some(3),
    abilities: &[triggered(when_asked(EventMask::DAMAGE, dealt_to_you), nothing,
        "Whenever you're dealt damage, you may create that many 1/1 white Soldier creature tokens.")],
    ..CardDef::BLANK
};

// Darien's printed trigger condition without the separate history-boundary probe's assumptions about life totals.
fn review_dealt_to_you(_: &dyn Query, me: &Asker, event: &Event) -> Option<Fired> {
    if matches!(event, Event::Damage { target: Target::Player(p), .. } if *p == me.controller) {
        Fired::PLAIN
    } else {
        None
    }
}

static REVIEW_DARIEN: CardDef = CardDef {
    name: "Damage Timing Darien, King of Kjeldor",
    abilities: &[triggered(when_asked(EventMask::DAMAGE, review_dealt_to_you), nothing,
        "Whenever you're dealt damage, you may create that many 1/1 white Soldier creature tokens.")],
    ..DARIEN
};

fn review_counters_on_your_permanent(_: &dyn Query, me: &Asker, proposal: &Proposal) -> bool {
    matches!(proposal, Proposal::Counters(c) if c.effect && c.controller == me.controller)
}

fn review_double_counters(_: &dyn Query, _: &Asker, proposal: &mut Proposal) {
    if let Proposal::Counters(c) = proposal {
        for amount in &mut c.counts {
            *amount *= 2;
        }
    }
}

/// Doubling Season's counter replacement; its token replacement is irrelevant to this combat.
static REVIEW_SEASON: CardDef = CardDef {
    name: "Damage Result Doubling Season",
    types: TypeSet::of(&[CardType::Enchantment]),
    replacements: &[replacement(ReplacementKind::Counters, review_counters_on_your_permanent, review_double_counters,
        "If an effect would put one or more counters on a permanent you control, it puts twice that many of those counters on that permanent instead.")],
    ..CardDef::BLANK
};

static REVIEW_STINGER: CardDef = CardDef {
    name: "Damage Result Plague Stinger",
    keywords: Keywords::of(&[Keyword::Flying, Keyword::Infect]),
    ..INFECT
};

fn result_loss(q: &dyn Query, me: &Asker, event: &Event) -> Option<Fired> {
    let fired = opponent_lost_life(q, me, event)?;
    assert!(q.state().players.iter().all(|p| p.life == 19), "life triggers see every applied result");
    assert!(q.state().battlefield().iter().filter(|&&id| q.chars(id).is(CardType::Creature))
        .all(|&id| q.state().obj(id).damage == 1), "life triggers also see the final damage marks");
    assert!(!q.history().records.iter().any(|r| matches!(r.event, Event::Damage { .. } | Event::LifeChange { .. })),
        "result triggers use the damage trigger's pre-instruction history boundary");
    Some(fired)
}

/// Mindcrank's trigger, with the complementary post-result-state/pre-instruction-history probe.
static RESULT_HISTORY_WATCHER: CardDef = CardDef {
    name: "Damage Result History Mindcrank",
    types: TypeSet::of(&[CardType::Artifact]),
    abilities: &[triggered(when_asked(EventMask::LIFE_CHANGE, result_loss), nothing,
        "Whenever an opponent loses life, that player mills that many cards.")],
    ..CardDef::BLANK
};

fn red_source_damage(q: &dyn Query, me: &Asker, event: &Event) -> Option<Fired> {
    if let Event::Damage { source, target, kind, controller, .. } = *event
        && controller == me.controller && kind.colors().has(Color::Red)
    {
        assert!(q.state().players.iter().all(|p| p.life == 20), "one-or-more damage triggers before results too");
        Some(Fired { subject: target, data: [0, source.0] })
    } else {
        None
    }
}

static RED_TERROR: CardDef = CardDef {
    name: "Damage Timing The Red Terror",
    types: TypeSet::of(&[CardType::Creature]),
    power: Some(4), toughness: Some(3),
    abilities: &[triggered(when_asked(EventMask::DAMAGE, red_source_damage), nothing,
        "Whenever a red source you control deals damage to one or more permanents and/or players, put a +1/+1 counter on The Red Terror.")
        .once_per_batch().batch_by_data()],
    ..CardDef::BLANK
};

fn result_counters(q: &dyn Query, me: &Asker, event: &Event) -> Option<Fired> {
    if let Event::CountersPut { object, kind: CounterKind::MinusOne, by, .. } = *event
        && by == me.controller && q.chars(object).is(CardType::Creature)
    {
        assert_eq!(q.state().player(ACTIVE).life, 23);
        assert_eq!(q.state().player(FIRST).life, 16);
        assert_eq!(q.state().player(LAST).poison(), 4);
        assert_eq!(q.state().obj(object).counters(CounterKind::MinusOne), 4);
        assert!(!q.history().records.iter().any(|r| matches!(r.event,
            Event::Damage { .. } | Event::LifeChange { .. } | Event::CountersPut { .. })),
            "counter-result matchers also exclude the whole instruction's history");
        Fired::about(Target::Object(object))
    } else {
        None
    }
}

static HAPATRA: CardDef = CardDef {
    name: "Damage Result Hapatra, Vizier of Poisons",
    types: TypeSet::of(&[CardType::Creature]),
    power: Some(2), toughness: Some(2),
    abilities: &[triggered(when_asked(EventMask::COUNTERS, result_counters), nothing,
        "Whenever you put one or more -1/-1 counters on a creature, create a 1/1 green Snake creature token with deathtouch.")
        .once_per_batch()],
    ..CardDef::BLANK
};

fn counters_put_by_you(_: &dyn Query, me: &Asker, proposal: &Proposal) -> bool {
    match proposal {
        Proposal::Counters(c) => c.putter == me.controller && c.counts.iter().any(|&n| n > 0),
        Proposal::PlayerCounters(c) => c.putter == me.controller && c.poison > 0,
        _ => false,
    }
}

fn double_placed_counters(q: &dyn Query, _: &Asker, proposal: &mut Proposal) {
    assert!(q.state().players.iter().all(|p| p.life == 20 && p.poison() == 0),
        "counter replacement choices precede all original life and player-counter results");
    assert!(q.state().battlefield().iter().all(|&id| q.state().obj(id).damage == 0
        && q.state().obj(id).counters(CounterKind::MinusOne) == 0),
        "counter replacements precede all marks and permanent-counter results");
    match proposal {
        Proposal::Counters(c) => c.counts.iter_mut().for_each(|n| *n *= 2),
        Proposal::PlayerCounters(c) => c.poison *= 2,
        _ => {}
    }
}

/// Innkeeper's Talent's level-three ability, already gained in these positions. The putter, not the recipient, owns it.
static TALENT: CardDef = CardDef {
    name: "Damage Result Level Three Innkeeper's Talent",
    types: TypeSet::of(&[CardType::Enchantment]),
    replacements: &[replacement(ReplacementKind::Counters, counters_put_by_you, double_placed_counters,
        "If you would put one or more counters on a permanent or player, put twice that many of each of those kinds of counters on that permanent or player instead.")],
    ..CardDef::BLANK
};

fn your_poison_banned(_: &dyn Query, me: &Asker, player: PlayerId, kind: PlayerCounter) -> bool {
    player == me.controller && kind == PlayerCounter::Poison
}

fn your_minus_one_banned(q: &dyn Query, me: &Asker, object: ObjectId, controller: PlayerId, kind: CounterKind) -> bool {
    controller == me.controller && kind == CounterKind::MinusOne && q.chars(object).is(CardType::Creature)
}

/// Melira's counter prohibitions. The only infect source in these tests is controlled by Melira's controller when
/// it deals damage, so the separate ability removing infect from opposing creatures cannot affect that damage.
static MELIRA: CardDef = CardDef {
    name: "Damage Result Melira, Sylvok Outcast",
    types: TypeSet::of(&[CardType::Creature]),
    power: Some(2), toughness: Some(2),
    player_counter_bans: &[your_poison_banned],
    counter_bans: &[your_minus_one_banned],
    ..CardDef::BLANK
};

fn wordsmith_opponent_damaged(q: &dyn LayerQuery, source: ObjectId, affected: ObjectId, _: &Chars) -> bool {
    source == affected && cantrip_mechanics::history::opponent_dealt_noncombat_damage_this_turn(
        q.history(), q.state(), q.controller(source))
}

/// Whiplash Wordsmith's history-dependent static, using the same history query as the printed card implementation.
static REVIEW_WORDSMITH: CardDef = CardDef {
    name: "Damage History Whiplash Wordsmith",
    types: TypeSet::of(&[CardType::Creature]),
    power: Some(3), toughness: Some(3),
    statics: &[static_ability(Layer::Ability, wordsmith_opponent_damaged,
        cantrip_mechanics::statics::have_flying_and_haste,
        "As long as an opponent was dealt noncombat damage this turn, this creature has flying and haste.")],
    ..CardDef::BLANK
};

fn sonic_damage(q: &dyn Query, me: &Asker, event: &Event) -> Option<Fired> {
    if let Event::Damage { target: Target::Object(id), .. } = *event {
        let chars = q.chars(id);
        if q.state().obj(id).controller == me.controller && chars.is(CardType::Creature)
            && (chars.has(Keyword::Flash) || chars.has(Keyword::Haste))
        {
            return Fired::about(Target::Object(id));
        }
    }
    None
}

/// Sonic's printed damage trigger; the attack trigger does not participate in these positions.
static REVIEW_SONIC: CardDef = CardDef {
    name: "Damage History Sonic the Hedgehog",
    types: TypeSet::of(&[CardType::Creature]),
    power: Some(2), toughness: Some(4),
    keywords: Keywords::of(&[Keyword::Haste]),
    abilities: &[triggered(when_asked(EventMask::DAMAGE, sonic_damage), nothing,
        "Whenever a creature you control with flash or haste is dealt damage, create a tapped Treasure token.")],
    ..CardDef::BLANK
};

fn review_double_all_counters(_: &dyn Query, _: &Asker, proposal: &mut Proposal) {
    match proposal {
        Proposal::Counters(c) => c.counts.iter_mut().for_each(|n| *n *= 2),
        Proposal::PlayerCounters(c) => { c.poison *= 2; c.energy *= 2; c.experience *= 2; }
        _ => {}
    }
}

static REVIEW_TALENT: CardDef = CardDef {
    name: "Counter Order Level Three Innkeeper's Talent",
    replacements: &[replacement(ReplacementKind::Counters, counters_put_by_you, review_double_all_counters,
        "If you would put one or more counters on a permanent or player, put twice that many of each of those kinds of counters on that permanent or player instead.")],
    ..TALENT
};

fn clasp_proliferate(g: &mut dyn cantrip_api::Game, item: &StackItem) -> Result<(), Suspend> {
    g.proliferate(item.controller, item.object)
}

static REVIEW_CLASP: CardDef = CardDef {
    name: "Counter Order Contagion Clasp",
    types: TypeSet::of(&[CardType::Artifact]),
    abilities: &[ability(Cost { tap: true, ..cost("{4}") }, clasp_proliferate, "{4}, {T}: Proliferate.")],
    ..CardDef::BLANK
};

fn engine_proliferate(g: &mut dyn cantrip_api::Game, item: &StackItem) -> Result<(), Suspend> {
    g.proliferate(item.controller, item.object)?;
    g.proliferate(item.controller, item.object)
}

static REVIEW_ENGINE: CardDef = CardDef {
    name: "Counter Order Contagion Engine",
    abilities: &[ability(Cost { tap: true, ..cost("{4}") }, engine_proliferate, "{4}, {T}: Proliferate, then proliferate again.")],
    ..REVIEW_CLASP
};

fn thrummingbird_damage(_: &dyn Query, me: &Asker, event: &Event) -> Option<Fired> {
    match *event {
        Event::Damage { source, target: Target::Player(_), kind, .. } if source == me.source && kind.is_combat() => Fired::PLAIN,
        _ => None,
    }
}

static REVIEW_THRUMMINGBIRD: CardDef = CardDef {
    name: "Counter Order Thrummingbird",
    types: TypeSet::of(&[CardType::Creature]),
    power: Some(1), toughness: Some(1), keywords: Keywords::of(&[Keyword::Flying]),
    abilities: &[triggered(when_asked(EventMask::DAMAGE, thrummingbird_damage), clasp_proliferate,
        "Whenever Thrummingbird deals combat damage to a player, proliferate.")],
    ..CardDef::BLANK
};

fn game() -> Game {
    let table = Box::leak(Box::new(CardTable::new([
        &WATCHER, &DOUBLER, &EMPERION, &REMEDY, &DRAIN, &PYROHEMIA, &LIFELINKER, &PRIDEMATE, &ANGEL, &DARIEN,
        &HAWK, &INFECT, &TOXIC, &ARMOR, &REVIEW_DARIEN, &REVIEW_SEASON, &REVIEW_STINGER,
        &RESULT_HISTORY_WATCHER, &TALENT, &MELIRA, &RED_TERROR, &HAPATRA, &REVIEW_WORDSMITH, &REVIEW_SONIC,
        &REVIEW_TALENT, &REVIEW_CLASP, &REVIEW_ENGINE, &REVIEW_THRUMMINGBIRD,
        &LAND_PAYMENT_LAND, &LAND_EXILER, &MIXED_PAYMENT, &SAME_PAYMENT, &OPTIONAL_LAND_EXILER, &CARD_COMPONENT_EXILER, &CONTROL_ON_LOSS, &CREATE_EXILERS_ON_LOSS, &LAND_EXILER_WATCHER, &TWO_PAYMENTS, &ASK_CONTROLLER_ON_LOSS, &MANDATORY_LAND_WITH_THEN,
    ])));
    let mut g = Game::new(table, &Setup { decks: vec![Vec::new(); 4], ..Default::default() });
    while !g.state.agenda.is_empty() {
        g.state.frame_remove(0);
    }
    for player in [YOU, FIRST, ACTIVE, LAST] {
        g.state.update_player(player, |p| p.drew_from_empty = false);
    }
    g.state.update_turn(|t| {
        t.active = ACTIVE;
        t.priority = ACTIVE;
        t.set_step(Step::PrecombatMain);
    });
    g.state.frame_push(Frame::new(FrameKind::Priority));
    g
}

fn put(g: &mut Game, def: &'static CardDef, controller: PlayerId) -> ObjectId {
    let id = g.state.push_obj(Obj::new(g.cards.id_of(def), controller, Zone::BATTLEFIELD));
    let timestamp = g.state.timestamp();
    g.state.update_obj(id, |o| o.timestamp = timestamp);
    g.state.zone_push(Zone::BATTLEFIELD, id);
    g.state.see_statics();
    id
}

fn cx(g: &mut Game) -> Cx<'_> {
    Cx { g, frame: 0, cursor: 0, mana_source: None, entering: None, lineage: Vec::new(), damage: None, asked_at: None, doer: None, producing: None }
}

fn changes(g: &Game) -> Vec<(PlayerId, i32)> {
    g.log.iter().filter_map(|r| match r.event {
        Event::LifeChange { player, delta } => Some((player, delta)),
        _ => None,
    }).collect()
}

fn life(g: &Game) -> Vec<i32> {
    g.state.players.iter().map(|p| p.life).collect()
}

fn trigger_count(g: &Game, source: ObjectId, index: u8) -> usize {
    g.state.triggers.iter().filter(|t| t.source == source && t.ability.index == index).count()
}

fn pyrohemia_item(g: &Game, source: ObjectId) -> StackItem {
    let card = g.state.obj(source).card;
    crate::actions::stack_item(StackKind::Activated, source, AbilityRef { card, index: 0 }, g.state.obj(source).controller, &[])
}

#[test]
fn pyrohemia_triggers_ob_nixilis_once_for_three_opponents_losing_one() {
    let mut g = game();
    let watcher = put(&mut g, &WATCHER, YOU);
    let source = put(&mut g, &PYROHEMIA, YOU);
    let item = pyrohemia_item(&g, source);
    pyrohemia(&mut cx(&mut g), &item).unwrap();
    assert_eq!(life(&g), vec![19; 4]);
    assert_eq!(changes(&g), vec![(ACTIVE, -1), (LAST, -1), (YOU, -1), (FIRST, -1)]);
    assert_eq!(trigger_count(&g, watcher, 0), 1, "one simultaneous loss event, not three triggers");
    assert_eq!(trigger_count(&g, watcher, 1), 3, "Mindcrank still sees each opponent's occurrence");
}

/// CR 120.4b, 603.10, 611.3a: the damage has been dealt when its triggers are checked. Wordsmith's static
/// therefore gives it haste from the opponent's simultaneous damage, and Sonic triggers for it as well as itself.
#[test]
fn damage_trigger_layers_see_the_simultaneous_damage_history() {
    let mut g = game();
    g.state.update_turn(|t| t.number = 1);
    let sonic = put(&mut g, &REVIEW_SONIC, YOU);
    let wordsmith = put(&mut g, &REVIEW_WORDSMITH, YOU);
    let source = put(&mut g, &PYROHEMIA, YOU);
    assert!(!cx(&mut g).chars(wordsmith).has(Keyword::Haste));
    let item = pyrohemia_item(&g, source);
    pyrohemia(&mut cx(&mut g), &item).unwrap();
    assert!(cx(&mut g).chars(wordsmith).has(Keyword::Haste));
    assert_eq!(trigger_count(&g, sonic, 0), 2, "Sonic sees itself and Wordsmith with haste from this damage");
}

#[test]
fn damage_history_and_layer_dependent_triggers_survive_result_choice_replay() {
    let mut g = game();
    g.state.update_turn(|t| t.number = 1);
    let sonic = put(&mut g, &REVIEW_SONIC, YOU);
    let wordsmith = put(&mut g, &REVIEW_WORDSMITH, YOU);
    let source = put(&mut g, &PYROHEMIA, YOU);
    put(&mut g, &DOUBLER, ACTIVE);
    put(&mut g, &DOUBLER, ACTIVE);
    g.state.stack_push(pyrohemia_item(&g, source));
    let serial = g.state.stack.last().unwrap().serial;
    g.state.frame_push(Frame::new(FrameKind::Resolve { serial }));
    g.run();
    let before = g.at;
    for player in [LAST, YOU, FIRST] {
        no_damage_results_yet(&g);
        assert_eq!(g.log.iter().filter(|r| matches!(r.event, Event::Damage { .. })).count(), 6);
        assert!(cx(&mut g).chars(wordsmith).has(Keyword::Haste));
        assert_eq!(trigger_count(&g, sonic, 0), 2, "both triggers exist before every result choice");
        let request = g.pending().cloned().unwrap();
        assert_eq!(request.player, player);
        g.answer(request.id, player, Answer::Index(0)).unwrap();
        g.run();
    }
    let order = g.pending().cloned().expect("Sonic's two already-triggered abilities need ordering");
    assert!(matches!(&order.kind, RequestKind::OrderTriggers { sources, .. } if *sources == vec![sonic, sonic]));
    g.answer(order.id, YOU, Answer::Order(vec![0, 1])).unwrap();
    g.run();
    let after = g.at;
    let completed = g.log.clone();
    assert_eq!(g.log.iter().filter(|r| matches!(r.event, Event::Damage { .. })).count(), 6);
    assert_eq!(changes(&g).len(), 4);
    assert_eq!(waiting_or_stacked(&g, sonic, YOU), 2);
    g.goto(before);
    no_damage_results_yet(&g);
    assert_eq!(g.log.iter().filter(|r| matches!(r.event, Event::Damage { .. })).count(), 6);
    assert_eq!(trigger_count(&g, sonic, 0), 2);
    g.goto(after);
    assert_eq!(g.log, completed, "damage is recorded once, and result records follow only on completion");
    assert_eq!(waiting_or_stacked(&g, sonic, YOU), 2);
}

/// Positive control: the same real static and trigger work when earlier noncombat damage gave Wordsmith haste.
#[test]
fn damage_trigger_layers_see_prior_damage_history_control() {
    let mut g = game();
    g.state.update_turn(|t| t.number = 1);
    let sonic = put(&mut g, &REVIEW_SONIC, YOU);
    let wordsmith = put(&mut g, &REVIEW_WORDSMITH, YOU);
    let source = put(&mut g, &PYROHEMIA, YOU);
    crate::ops::damage(&mut cx(&mut g), source, Target::Player(FIRST), 1, false);
    assert!(cx(&mut g).chars(wordsmith).has(Keyword::Haste));
    let item = pyrohemia_item(&g, source);
    pyrohemia(&mut cx(&mut g), &item).unwrap();
    assert_eq!(trigger_count(&g, sonic, 0), 2);
}

/// Negative control: an ordinary one-damage spell hitting only Wordsmith gives it no haste and triggers no Sonic.
#[test]
fn damage_trigger_layers_without_opponent_damage_control() {
    let mut g = game();
    g.state.update_turn(|t| t.number = 1);
    let sonic = put(&mut g, &REVIEW_SONIC, YOU);
    let wordsmith = put(&mut g, &REVIEW_WORDSMITH, YOU);
    let source = put(&mut g, &PYROHEMIA, YOU);
    crate::ops::damage(&mut cx(&mut g), source, Target::Object(wordsmith), 1, false);
    assert!(!cx(&mut g).chars(wordsmith).has(Keyword::Haste));
    assert_eq!(trigger_count(&g, sonic, 0), 0);
}

#[test]
fn one_or_more_damage_triggers_keep_the_whole_batch() {
    let mut g = game();
    let watcher = put(&mut g, &RED_TERROR, YOU);
    let source = put(&mut g, &PYROHEMIA, YOU);
    let item = pyrohemia_item(&g, source);
    pyrohemia(&mut cx(&mut g), &item).unwrap();
    assert_eq!(trigger_count(&g, watcher, 0), 1, "one per red source, not one per hit or one per result phase");
    assert_eq!(life(&g), vec![19; 4]);
    assert_eq!(g.log.iter().filter(|r| matches!(r.event, Event::Damage { .. })).count(), 5);
}

#[test]
fn damage_and_its_life_results_share_the_same_trigger_history_boundary() {
    let mut g = game();
    let darien = put(&mut g, &DARIEN, FIRST);
    let result_watcher = put(&mut g, &RESULT_HISTORY_WATCHER, YOU);
    let source = put(&mut g, &PYROHEMIA, YOU);
    let item = pyrohemia_item(&g, source);
    pyrohemia(&mut cx(&mut g), &item).unwrap();
    assert_eq!(trigger_count(&g, darien, 0), 1);
    assert_eq!(trigger_count(&g, result_watcher, 0), 3);
    assert_eq!(g.state.obj(darien).damage, 1);
}

/// CR 510.2, 120.3a: two simultaneous one-damage assignments to one player cause one loss of two, not two losses of one.
#[test]
fn combat_damage_combines_a_players_losses_but_not_sources_lifelink() {
    let mut g = game();
    let watcher = put(&mut g, &WATCHER, YOU);
    let pridemate = put(&mut g, &PRIDEMATE, ACTIVE);
    let one = put(&mut g, &LIFELINKER, ACTIVE);
    let two = put(&mut g, &LIFELINKER, ACTIVE);
    let hits = [(one, Target::Player(FIRST), 1), (two, Target::Player(FIRST), 1)];
    let mut c = cx(&mut g);
    let mut batch = crate::ops::DamageBatch::new(&c, &hits);
    let mut events: Vec<_> = hits.into_iter()
        .filter_map(|(s, t, n)| crate::ops::deal_damage_with(&mut c, s, t, n, true, &mut batch)).collect();
    assert_eq!(life(c.g), vec![20; 4], "assignment preparation does not change life");
    batch.finish(&mut c, &mut events);
    batch.rest_of_preventions(&mut c);
    assert_eq!(changes(&g), vec![(ACTIVE, 1), (ACTIVE, 1), (FIRST, -2)]);
    assert_eq!(trigger_count(&g, watcher, 0), 0);
    assert_eq!(trigger_count(&g, watcher, 1), 1);
    assert_eq!(trigger_count(&g, pridemate, 0), 2, "CR 702.15e: one gain per source, not per controller");
}

#[test]
fn lifelink_prepares_one_gain_per_source_across_all_its_recipients() {
    let mut g = game();
    let pridemate = put(&mut g, &PRIDEMATE, ACTIVE);
    put(&mut g, &ANGEL, ACTIVE);
    put(&mut g, &DOUBLER, ACTIVE);
    let one = put(&mut g, &LIFELINKER, ACTIVE);
    let two = put(&mut g, &LIFELINKER, ACTIVE);
    assert_eq!(cantrip_api::Game::damage_together(&mut cx(&mut g), &[
        (one, Target::Player(FIRST), 1), (one, Target::Player(LAST), 1), (two, Target::Player(FIRST), 1),
    ]), vec![1, 1, 1]);
    assert_eq!(changes(&g), vec![(ACTIVE, 3), (ACTIVE, 2), (LAST, -2), (FIRST, -4)]);
    assert_eq!(life(&g), vec![20, 16, 25, 18]);
    assert_eq!(trigger_count(&g, pridemate, 0), 2);
}

#[test]
fn single_damage_keeps_loss_and_lifelink_separate_even_at_unchanged_life() {
    let mut g = game();
    let watcher = put(&mut g, &WATCHER, YOU);
    let pridemate = put(&mut g, &PRIDEMATE, FIRST);
    let source = put(&mut g, &LIFELINKER, FIRST);
    assert_eq!(cantrip_api::Game::damage_result(&mut cx(&mut g), source, Target::Player(FIRST), 1),
        Some(DamageResult { target: Target::Player(FIRST), amount: 1, excess: 0 }));
    assert_eq!(life(&g), vec![20; 4]);
    assert_eq!(changes(&g), vec![(FIRST, -1), (FIRST, 1)], "do not net a loss against a gain");
    assert_eq!(trigger_count(&g, watcher, 0), 1);
    assert_eq!(trigger_count(&g, pridemate, 0), 1);
}

#[test]
fn damage_life_choices_precede_all_updates_and_survive_undo_redo() {
    let mut g = game();
    let one = put(&mut g, &DOUBLER, ACTIVE);
    let two = put(&mut g, &DOUBLER, ACTIVE);
    let source = put(&mut g, &PYROHEMIA, YOU);
    g.state.stack_push(pyrohemia_item(&g, source));
    let serial = g.state.stack.last().unwrap().serial;
    g.state.frame_push(Frame::new(FrameKind::Resolve { serial }));
    g.run();
    let before = g.at;
    for player in [LAST, YOU, FIRST] {
        let request = g.pending().cloned().expect("damage-result replacement question");
        assert_eq!(request.player, player, "life results are prepared in APNAP order");
        let RequestKind::ChooseReplacement { life: amount, sources, .. } = &request.kind else { panic!("{:?}", request.kind) };
        assert_eq!(*amount, Some(-1));
        assert_eq!(*sources, vec![one, two]);
        assert_eq!(life(&g), vec![20; 4]);
        assert!(changes(&g).is_empty());
        assert_eq!(g.log.iter().filter(|r| matches!(r.event, Event::Damage { .. })).count(), 4,
            "damage is already dealt and recorded, without duplicate records on replay");
        g.answer(request.id, player, Answer::Index(1)).expect("valid order");
        g.run();
    }
    let after = g.at;
    let expected = vec![(ACTIVE, -1), (LAST, -4), (YOU, -4), (FIRST, -4)];
    assert_eq!(changes(&g), expected);
    assert_eq!(life(&g), vec![16, 16, 19, 16]);
    assert_eq!(g.log.iter().filter(|r| matches!(r.event, Event::Damage { .. })).count(), 4);
    g.goto(before);
    assert_eq!(g.pending().unwrap().player, LAST);
    assert_eq!(life(&g), vec![20; 4]);
    assert!(changes(&g).is_empty());
    g.goto(after);
    assert_eq!(changes(&g), expected);
    assert_eq!(life(&g), vec![16, 16, 19, 16]);
    assert_eq!(g.log.iter().filter(|r| matches!(r.event, Event::Damage { .. })).count(), 4);
}

#[test]
fn damage_result_replacement_choices_rebuild_after_concession() {
    let mut g = game();
    put(&mut g, &DOUBLER, ACTIVE);
    put(&mut g, &DOUBLER, ACTIVE);
    let source = put(&mut g, &PYROHEMIA, YOU);
    g.state.stack_push(pyrohemia_item(&g, source));
    let serial = g.state.stack.last().unwrap().serial;
    g.state.frame_push(Frame::new(FrameKind::Resolve { serial }));
    g.run();
    assert_eq!(g.pending().unwrap().player, LAST);
    assert!(matches!(g.pending().unwrap().kind, RequestKind::ChooseReplacement { .. }));
    g.concede(ACTIVE);
    assert!(!matches!(g.pending().map(|r| &r.kind), Some(RequestKind::ChooseReplacement { .. })));
    assert_eq!(changes(&g), vec![(LAST, -1), (YOU, -1), (FIRST, -1)]);
    assert_eq!(life(&g), vec![19, 19, 20, 19]);
    // CR 120.4b-c, 104.3a: all four assignments were dealt before this life-result choice. Concession preserves that
    // damage, but the departing player has no subsequent life change. Replay must not move the concession earlier.
    let recipients: Vec<_> = g.log.iter().filter_map(|r| match r.event {
        Event::Damage { target, .. } => Some(target),
        _ => None,
    }).collect();
    assert_eq!(recipients, vec![Target::Player(ACTIVE), Target::Player(LAST), Target::Player(YOU), Target::Player(FIRST)]);
}

/// CR 120.4b-c: Darien has already triggered when damage's life-loss replacements are chosen. Its owner leaving
/// then removes the permanent (CR 800.4a), not the surviving controller's trigger (CR 603.3a, 800.4d).
/// Reachable with Bribery taking Darien, two Bloodletters controlled by ACTIVE, and YOU activating Pyrohemia.
#[test]
fn darien_trigger_survives_owner_conceding_during_damage_life_replacements() {
    for concede_owner in [false, true] {
        let mut g = game();
        let darien = put(&mut g, &REVIEW_DARIEN, ACTIVE);
        // Bribery puts the opponent's card onto the battlefield under YOU's control; ownership is unchanged.
        g.state.update_obj(darien, |o| o.controller = YOU);
        g.state.see_statics();
        put(&mut g, &DOUBLER, ACTIVE);
        put(&mut g, &DOUBLER, ACTIVE);
        let source = put(&mut g, &PYROHEMIA, YOU);
        g.state.stack_push(pyrohemia_item(&g, source));
        let serial = g.state.stack.last().unwrap().serial;
        g.state.frame_push(Frame::new(FrameKind::Resolve { serial }));
        g.run();
        let request = g.pending().unwrap();
        assert_eq!(request.player, LAST);
        assert!(matches!(request.kind, RequestKind::ChooseReplacement { life: Some(-1), .. }));
        assert_eq!(g.state.obj(darien).owner, ACTIVE);
        assert_eq!(g.state.obj(darien).controller, YOU);
        assert_eq!(trigger_count(&g, darien, 0), 1, "already triggered before the first result choice");
        assert_eq!(g.state.obj(darien).damage, 0, "damage marks wait for all result choices");

        if concede_owner {
            g.concede(ACTIVE);
            assert!(!g.state.in_game(ACTIVE));
            assert!(!g.state.battlefield().contains(&darien));
            assert_eq!(changes(&g), vec![(LAST, -1), (YOU, -1), (FIRST, -1)]);
        } else {
            // Positive control: the same printed trigger is collected when its owner stays.
            for player in [LAST, YOU, FIRST] {
                let request = g.pending().cloned().unwrap();
                assert_eq!(request.player, player);
                g.answer(request.id, player, Answer::Index(0)).unwrap();
                g.run();
            }
        }
        assert!(g.state.in_game(YOU));
        let pending = g.state.triggers.iter()
            .filter(|t| t.source == darien && t.ability.index == 0 && t.controller == YOU).count();
        let stacked = g.state.stack.iter()
            .filter(|item| item.kind == StackKind::Triggered && item.object == darien && item.controller == YOU).count();
        assert_eq!(pending + stacked, 1,
            "Darien triggered at CR 120.4b and belongs to YOU even if its owner later leaves; concede_owner={concede_owner}");
    }
}

/// CR 510.2, 120.4c, 616.1: results of the same combat damage are replaced in APNAP order, not all life choices
/// before all counter choices. ACTIVE's Vampire Nighthawk is blocked by FIRST's Plague Stinger, while ACTIVE's
/// Healer's Hawk hits FIRST. Two Doubling Seasons modify ACTIVE's -1/-1 counters; two Bloodletters modify FIRST's loss.
#[test]
fn damage_result_choices_are_apnap_across_counters_and_life() {
    for with_life_choices in [false, true] {
        let mut g = game();
        let first_season = put(&mut g, &REVIEW_SEASON, ACTIVE);
        let second_season = put(&mut g, &REVIEW_SEASON, ACTIVE);
        if with_life_choices {
            put(&mut g, &DOUBLER, ACTIVE);
            put(&mut g, &DOUBLER, ACTIVE);
        }
        let nighthawk = put(&mut g, &LIFELINKER, ACTIVE);
        let hawk = put(&mut g, &HAWK, ACTIVE);
        let stinger = put(&mut g, &REVIEW_STINGER, FIRST);
        g.state.set_attackers(vec![
            Attacker { blocked: true, ..Attacker::new(nighthawk, Target::Player(FIRST), FIRST, TypeSet::NONE) },
            Attacker::new(hawk, Target::Player(FIRST), FIRST, TypeSet::NONE),
        ]);
        g.state.set_blocks(vec![Block { blocker: stinger, attacker: nighthawk, first_strike: false }]);
        g.state.update_turn(|t| t.set_step(Step::CombatDamage));
        g.state.frame_push(Frame::new(FrameKind::BeginStep));
        g.run();
        let request = g.pending().expect("damage-result replacement order");
        assert_eq!(request.player, ACTIVE,
            "ACTIVE's counter-result choices precede FIRST's life-result choices; with_life_choices={with_life_choices}");
        let RequestKind::ChooseReplacement { life: None, counters, sources, .. } = &request.kind else {
            panic!("expected the active player's counter-result order: {:?}", request.kind);
        };
        assert_eq!(*counters, vec![(CounterKind::MinusOne, 1)]);
        assert_eq!(*sources, vec![first_season, second_season]);
        assert_eq!(life(&g), vec![20; 4]);
        assert_eq!(g.state.obj(nighthawk).counters(CounterKind::MinusOne), 0);
    }
}

fn no_damage_results_yet(g: &Game) {
    assert_eq!(life(g), vec![20; 4]);
    assert!(g.state.players.iter().all(|p| p.poison() == 0));
    assert!(g.state.battlefield().iter().all(|&id| g.state.obj(id).damage == 0
        && g.state.obj(id).counters(CounterKind::MinusOne) == 0));
    assert!(g.log.iter().any(|r| matches!(r.event, Event::Damage { .. })), "dealt damage is already recorded");
    assert!(!g.log.iter().any(|r| matches!(r.event, Event::LifeChange { .. }
        | Event::CountersPut { kind: CounterKind::MinusOne, .. })), "no damage results have occurred yet");
}

fn waiting_or_stacked(g: &Game, source: ObjectId, controller: PlayerId) -> usize {
    g.state.triggers.iter().filter(|t| t.source == source && t.controller == controller).count()
        + g.state.stack.iter().filter(|i| i.kind == StackKind::Triggered && i.object == source && i.controller == controller).count()
}

/// One real combat has all three result kinds: counter placement on ACTIVE, poison on LAST, life loss on FIRST.
/// The attack order proposes LAST's poison first, which must not jump ahead of ACTIVE's permanent counters.
#[test]
fn damage_results_share_apnap_across_permanent_counters_poison_and_life_with_replay() {
    let mut g = game();
    let seasons = [put(&mut g, &REVIEW_SEASON, ACTIVE), put(&mut g, &REVIEW_SEASON, ACTIVE)];
    let talents = [put(&mut g, &TALENT, ACTIVE), put(&mut g, &TALENT, ACTIVE)];
    let bloodletters = [put(&mut g, &DOUBLER, ACTIVE), put(&mut g, &DOUBLER, ACTIVE)];
    let nighthawk = put(&mut g, &LIFELINKER, ACTIVE);
    let hawk = put(&mut g, &HAWK, ACTIVE);
    let infect = put(&mut g, &INFECT, ACTIVE);
    let stinger = put(&mut g, &REVIEW_STINGER, FIRST);
    let darien = put(&mut g, &REVIEW_DARIEN, LAST);
    let hapatra = put(&mut g, &HAPATRA, FIRST);
    g.state.set_attackers(vec![
        Attacker::new(infect, Target::Player(LAST), LAST, TypeSet::NONE),
        Attacker::new(hawk, Target::Player(FIRST), FIRST, TypeSet::NONE),
        Attacker { blocked: true, ..Attacker::new(nighthawk, Target::Player(FIRST), FIRST, TypeSet::NONE) },
    ]);
    g.state.set_blocks(vec![Block { blocker: stinger, attacker: nighthawk, first_strike: false }]);
    g.state.update_turn(|t| t.set_step(Step::CombatDamage));
    g.state.frame_push(Frame::new(FrameKind::BeginStep));
    g.run();
    let before = g.at;
    for (player, sources, loss, permanent) in [
        (ACTIVE, seasons, None, Some(nighthawk)),
        (LAST, talents, None, None),
        (FIRST, bloodletters, Some(-1), None),
    ] {
        let request = g.pending().cloned().unwrap();
        assert_eq!(request.player, player);
        let RequestKind::ChooseReplacement { sources: offered, life, subject, .. } = request.kind else { panic!("expected result replacement") };
        assert_eq!(offered, sources);
        assert_eq!(life, loss);
        assert_eq!(subject, permanent);
        no_damage_results_yet(&g);
        assert_eq!(trigger_count(&g, darien, 0), 1, "poison damage already triggered, exactly once");
        assert_eq!(trigger_count(&g, hapatra, 0), 0, "counter-result triggers wait until results occur");
        g.answer(request.id, player, Answer::Index(1)).unwrap();
        g.run();
    }
    let after = g.at;
    assert_eq!(life(&g), vec![20, 16, 23, 20]);
    assert_eq!(g.state.player(LAST).poison(), 4);
    assert!(g.log.iter().any(|r| matches!(r.event,
        Event::CountersPut { object, kind: CounterKind::MinusOne, amount: 4, by: FIRST, .. } if object == nighthawk)));
    assert_eq!(g.log.iter().filter(|r| matches!(r.event, Event::Damage { .. })).count(), 4);
    assert_eq!(waiting_or_stacked(&g, darien, LAST), 1);
    assert_eq!(waiting_or_stacked(&g, hapatra, FIRST), 1);
    let completed = g.log.clone();
    g.goto(before);
    no_damage_results_yet(&g);
    assert_eq!(g.pending().unwrap().player, ACTIVE);
    assert_eq!(trigger_count(&g, darien, 0), 1);
    g.goto(after);
    assert_eq!(g.log, completed, "undo/redo keeps completed damage and results without duplicates");
    assert_eq!(g.state.player(LAST).poison(), 4);
    assert_eq!(waiting_or_stacked(&g, darien, LAST), 1);
}

/// The opposite cross-kind order: ACTIVE's lifelink replacements must precede FIRST's poison replacements, even
/// though the infect source is the first attacker. Also covers toxic as an additional result, rather than infect.
#[test]
fn life_choices_precede_later_players_infect_and_toxic_counter_choices() {
    for poison_source in [&INFECT, &TOXIC] {
        let mut g = game();
        let angels = [put(&mut g, &ANGEL, ACTIVE), put(&mut g, &ANGEL, ACTIVE)];
        let talents = [put(&mut g, &TALENT, ACTIVE), put(&mut g, &TALENT, ACTIVE)];
        let poison = put(&mut g, poison_source, ACTIVE);
        let nighthawk = put(&mut g, &LIFELINKER, ACTIVE);
        let darien = put(&mut g, &REVIEW_DARIEN, FIRST);
        g.state.set_attackers(vec![
            Attacker::new(poison, Target::Player(FIRST), FIRST, TypeSet::NONE),
            Attacker::new(nighthawk, Target::Player(LAST), LAST, TypeSet::NONE),
        ]);
        g.state.update_turn(|t| t.set_step(Step::CombatDamage));
        g.state.frame_push(Frame::new(FrameKind::BeginStep));
        g.run();
        for (player, sources, gain) in [(ACTIVE, angels, Some(2)), (FIRST, talents, None)] {
            let request = g.pending().cloned().unwrap();
            assert_eq!(request.player, player);
            assert!(matches!(&request.kind, RequestKind::ChooseReplacement { life, sources: offered, .. }
                if *life == gain && *offered == sources));
            no_damage_results_yet(&g);
            assert_eq!(trigger_count(&g, darien, 0), 1);
            g.answer(request.id, player, Answer::Index(0)).unwrap();
            g.run();
        }
        assert_eq!(life(&g), vec![20, if poison_source.toxic > 0 { 19 } else { 20 }, 24, 18]);
        assert_eq!(g.state.player(FIRST).poison(), 4);
        assert_eq!(waiting_or_stacked(&g, darien, FIRST), 1);
    }
}

/// CR 120.3d: the same putter's two infect sources remain two placement occurrences, not a merged addition.
#[test]
fn simultaneous_infect_keeps_separate_source_counter_placements() {
    let mut g = game();
    let one = put(&mut g, &INFECT, ACTIVE);
    let two = put(&mut g, &INFECT, ACTIVE);
    let target = put(&mut g, &LIFELINKER, FIRST);
    put(&mut g, &TALENT, ACTIVE);
    crate::ops::damage_together(&mut cx(&mut g), &[
        (one, Target::Object(target), 1), (two, Target::Object(target), 1),
    ]);
    let counters: Vec<_> = g.log.iter().filter_map(|r| match r.event {
        Event::CountersPut { object, kind: CounterKind::MinusOne, amount, total, by, .. } if object == target => Some((amount, total, by)),
        _ => None,
    }).collect();
    assert_eq!(counters, vec![(2, 2, ACTIVE), (2, 4, ACTIVE)]);
    assert_eq!(g.state.obj(target).counters(CounterKind::MinusOne), 4);
    assert_eq!(g.log.iter().filter(|r| matches!(r.event, Event::Damage { recipient_excess: 0, .. })).count(), 2,
        "counter doubling changes results, not the actual damage or its excess");
}

/// A poison result already prepared for LAST is no longer allowed after the later life-result choice: ACTIVE
/// concedes, returning LAST's Melira that a control effect had taken. The dealt damage and Darien trigger remain.
#[test]
fn prepared_poison_rechecks_prohibition_after_a_later_result_choice_concession() {
    let mut g = game();
    put(&mut g, &TALENT, ACTIVE);
    put(&mut g, &TALENT, ACTIVE);
    put(&mut g, &DOUBLER, ACTIVE);
    put(&mut g, &DOUBLER, ACTIVE);
    let infect = put(&mut g, &INFECT, ACTIVE);
    let hawk = put(&mut g, &HAWK, ACTIVE);
    let melira = put(&mut g, &MELIRA, LAST);
    let darien = put(&mut g, &REVIEW_DARIEN, LAST);
    cantrip_api::Game::add_effect(&mut cx(&mut g), infect, melira, EffectKind::Control(ACTIVE), Duration::EndOfTurn);
    assert_eq!(g.state.obj(melira).controller, ACTIVE);
    g.state.set_attackers(vec![
        Attacker::new(infect, Target::Player(LAST), LAST, TypeSet::NONE),
        Attacker::new(hawk, Target::Player(FIRST), FIRST, TypeSet::NONE),
    ]);
    g.state.update_turn(|t| t.set_step(Step::CombatDamage));
    g.state.frame_push(Frame::new(FrameKind::BeginStep));
    g.run();
    let request = g.pending().cloned().unwrap();
    assert_eq!(request.player, LAST);
    assert_eq!(trigger_count(&g, darien, 0), 1);
    g.answer(request.id, LAST, Answer::Index(0)).unwrap();
    g.run();
    assert_eq!(g.pending().unwrap().player, FIRST);
    no_damage_results_yet(&g);
    g.concede(ACTIVE);
    assert_eq!(g.state.obj(melira).controller, LAST);
    assert_eq!(g.state.player(LAST).poison(), 0, "Melira now prohibits the prepared poison result");
    assert_eq!(g.state.player(FIRST).life, 19);
    assert_eq!(waiting_or_stacked(&g, darien, LAST), 1);
    assert_eq!(g.log.iter().filter(|r| matches!(r.event, Event::Damage { .. })).count(), 2);
    let known = g.log.iter().find_map(|r| match r.event {
        Event::Damage { source, controller: ACTIVE, known, .. } if source == infect => Some(known),
        _ => None,
    }).expect("damage retains its departed source's controller and LKI");
    assert!(g.state.last_known[known as usize].keywords.has(Keyword::Infect));
}

/// CR 101.4d: a concession during LAST's life choice returns a borrowed counter recipient to ACTIVE. Its still
/// unprepared counter replacements must restart APNAP, not be skipped because ACTIVE's earlier pass is over.
#[test]
fn result_preparation_restarts_apnap_when_concession_returns_a_counter_recipient() {
    let mut g = game();
    let seasons = [put(&mut g, &REVIEW_SEASON, ACTIVE), put(&mut g, &REVIEW_SEASON, ACTIVE)];
    put(&mut g, &DOUBLER, ACTIVE);
    put(&mut g, &DOUBLER, ACTIVE);
    let stinger = put(&mut g, &REVIEW_STINGER, ACTIVE);
    let hawk = put(&mut g, &HAWK, ACTIVE);
    let nighthawk = put(&mut g, &LIFELINKER, ACTIVE);
    let controller_source = put(&mut g, &PYROHEMIA, FIRST);
    cantrip_api::Game::add_effect(&mut cx(&mut g), controller_source, nighthawk, EffectKind::Control(FIRST), Duration::EndOfTurn);
    g.state.set_attackers(vec![
        Attacker { blocked: true, ..Attacker::new(stinger, Target::Player(FIRST), FIRST, TypeSet::NONE) },
        Attacker::new(hawk, Target::Player(LAST), LAST, TypeSet::NONE),
    ]);
    g.state.set_blocks(vec![Block { blocker: nighthawk, attacker: stinger, first_strike: false }]);
    g.state.update_turn(|t| t.set_step(Step::CombatDamage));
    g.state.frame_push(Frame::new(FrameKind::BeginStep));
    g.run();
    assert_eq!(g.pending().unwrap().player, LAST);
    g.concede(FIRST);
    // The outstanding LAST choice is rebuilt before the outer result scheduler resumes.
    let request = g.pending().cloned().unwrap();
    assert_eq!(request.player, LAST);
    g.answer(request.id, LAST, Answer::Index(0)).unwrap();
    g.run();
    let request = g.pending().cloned().expect("ACTIVE must order the returned creature's counters");
    assert_eq!(request.player, ACTIVE);
    assert!(matches!(&request.kind, RequestKind::ChooseReplacement { subject: Some(id), sources, .. }
        if *id == nighthawk && *sources == seasons));
    no_damage_results_yet(&g);
    g.answer(request.id, ACTIVE, Answer::Index(0)).unwrap();
    g.run();
    assert!(g.log.iter().any(|r| matches!(r.event,
        Event::CountersPut { object, kind: CounterKind::MinusOne, amount: 4, by: ACTIVE, .. } if object == nighthawk)));
    assert_eq!(g.state.player(LAST).life, 16);
    assert_eq!(g.state.player(ACTIVE).life, 21);
}

/// CR 701.34a, 616.1, 101.4: proliferating a player and a permanent is one simultaneous instruction.
/// ACTIVE's poison-counter replacements must precede FIRST's permanent-counter replacements, not vice versa.
#[test]
fn proliferate_counter_replacements_are_apnap_across_players_and_permanents() {
    let mut g = game();
    let nighthawk = put(&mut g, &LIFELINKER, FIRST);
    crate::counters::put(&mut cx(&mut g), nighthawk, CounterKind::PlusOne, 1, FIRST, crate::counters::How::Effect);
    crate::counters::put_on_player(&mut cx(&mut g), ACTIVE,
        crate::counters::PlayerCounts { poison: 1, ..Default::default() }, FIRST);
    put(&mut g, &REVIEW_TALENT, ACTIVE);
    put(&mut g, &REVIEW_TALENT, ACTIVE);
    let source = put(&mut g, &REVIEW_CLASP, ACTIVE);
    g.state.stack_push(pyrohemia_item(&g, source));
    let serial = g.state.stack.last().unwrap().serial;
    g.state.frame_push(Frame::new(FrameKind::Resolve { serial }));
    g.run();
    let request = g.pending().cloned().unwrap();
    assert_eq!(request.player, ACTIVE);
    assert!(matches!(request.kind, RequestKind::ChooseTarget { .. }));
    g.answer(request.id, ACTIVE, Answer::Targets(vec![Target::Player(ACTIVE), Target::Object(nighthawk)])).unwrap();
    g.run();
    let request = g.pending().unwrap();
    assert!(matches!(request.kind, RequestKind::ChooseReplacement { .. }));
    assert_eq!(request.player, ACTIVE, "proliferate's player counters cannot be scheduled after all permanent counters");
    assert_eq!(g.state.obj(nighthawk).counters(CounterKind::PlusOne), 1);
    assert_eq!(g.state.player(ACTIVE).poison(), 1);
}

/// CR 701.34a: every kind on a selected recipient is part of the same placement. Both input orders still ask the
/// active player first, and the original counters remain unchanged through both questions and undo/redo.
#[test]
fn proliferate_mixed_counter_kinds_are_prepared_together_and_replay_once() {
    for player_first in [false, true] {
        let mut g = game();
        let creature = put(&mut g, &LIFELINKER, FIRST);
        let mut initial = [0; CounterKind::COUNT];
        initial[CounterKind::PlusOne as usize] = 1;
        initial[CounterKind::Shield as usize] = 2;
        crate::counters::put_all(&mut cx(&mut g), creature, initial, FIRST, crate::counters::How::Effect);
        crate::counters::put_on_player(&mut cx(&mut g), ACTIVE,
            crate::counters::PlayerCounts { poison: 1, energy: 2, experience: 3 }, FIRST);
        put(&mut g, &REVIEW_TALENT, ACTIVE);
        put(&mut g, &REVIEW_TALENT, ACTIVE);
        let source = put(&mut g, &REVIEW_CLASP, ACTIVE);
        g.state.stack_push(pyrohemia_item(&g, source));
        let serial = g.state.stack.last().unwrap().serial;
        g.state.frame_push(Frame::new(FrameKind::Resolve { serial }));
        g.run();
        let request = g.pending().cloned().unwrap();
        let mut chosen = vec![Target::Object(creature), Target::Player(ACTIVE)];
        if player_first { chosen.reverse(); }
        g.answer(request.id, ACTIVE, Answer::Targets(chosen)).unwrap();
        g.run();
        let before = g.at;
        for player in [ACTIVE, FIRST] {
            let request = g.pending().cloned().unwrap();
            assert_eq!(request.player, player);
            assert!(matches!(request.kind, RequestKind::ChooseReplacement { .. }));
            let p = g.state.player(ACTIVE);
            assert_eq!((p.poison(), p.energy(), p.experience()), (1, 2, 3));
            assert_eq!(g.state.obj(creature).counters(CounterKind::PlusOne), 1);
            assert_eq!(g.state.obj(creature).counters(CounterKind::Shield), 2);
            assert_eq!(g.log.iter().filter(|r| matches!(r.event, Event::CountersPut { .. })).count(), 2);
            g.answer(request.id, player, Answer::Index(0)).unwrap();
            g.run();
        }
        let after = g.at;
        let completed = g.log.clone();
        let p = g.state.player(ACTIVE);
        assert_eq!((p.poison(), p.energy(), p.experience()), (5, 6, 7));
        assert_eq!(g.state.obj(creature).counters(CounterKind::PlusOne), 5);
        assert_eq!(g.state.obj(creature).counters(CounterKind::Shield), 6);
        assert_eq!(g.log.iter().filter(|r| matches!(r.event,
            Event::CountersPut { amount: 4, by: ACTIVE, .. })).count(), 2);
        g.goto(before);
        assert_eq!(g.pending().unwrap().player, ACTIVE);
        assert_eq!(g.state.player(ACTIVE).poison(), 1);
        assert_eq!(g.state.obj(creature).counters(CounterKind::PlusOne), 1);
        g.goto(after);
        assert_eq!(g.log, completed);
        assert_eq!(g.state.player(ACTIVE).experience(), 7);
        assert_eq!(g.state.obj(creature).counters(CounterKind::Shield), 6);
    }
}

/// The other printed proliferate callers with no scenarios in this checkout: Engine completes its first instruction
/// before asking the second selection; Thrummingbird waits for its damage trigger to resolve, then uses the same batch.
#[test]
fn proliferate_from_engine_and_combat_trigger_uses_complete_mixed_batches() {
    for (def, times, combat) in [(&REVIEW_ENGINE, 2u8, false), (&REVIEW_THRUMMINGBIRD, 1, true)] {
        let mut g = game();
        let creature = put(&mut g, &LIFELINKER, FIRST);
        crate::counters::put(&mut cx(&mut g), creature, CounterKind::PlusOne, 1, FIRST, crate::counters::How::Effect);
        crate::counters::put_on_player(&mut cx(&mut g), ACTIVE,
            crate::counters::PlayerCounts { poison: 1, ..Default::default() }, FIRST);
        put(&mut g, &REVIEW_TALENT, ACTIVE);
        put(&mut g, &REVIEW_TALENT, ACTIVE);
        let source = put(&mut g, def, ACTIVE);
        if combat {
            g.state.set_attackers(vec![Attacker::new(source, Target::Player(FIRST), FIRST, TypeSet::NONE)]);
            g.state.update_turn(|t| t.set_step(Step::CombatDamage));
            g.state.frame_push(Frame::new(FrameKind::BeginStep));
            g.run();
            for player in [ACTIVE, LAST, YOU, FIRST] {
                let request = g.pending().cloned().unwrap();
                assert_eq!(request.player, player);
                assert!(matches!(request.kind, RequestKind::Priority { .. }));
                g.answer(request.id, player, Answer::Pass).unwrap();
                g.run();
            }
        } else {
            g.state.stack_push(pyrohemia_item(&g, source));
            let serial = g.state.stack.last().unwrap().serial;
            g.state.frame_push(Frame::new(FrameKind::Resolve { serial }));
            g.run();
        }
        for round in 0..times {
            let current = 1 + 4 * round;
            let request = g.pending().cloned().unwrap();
            assert!(matches!(request.kind, RequestKind::ChooseTarget { .. }));
            assert_eq!(g.state.player(ACTIVE).poison(), current);
            assert_eq!(g.state.obj(creature).counters(CounterKind::PlusOne), i32::from(current));
            g.answer(request.id, ACTIVE, Answer::Targets(vec![Target::Object(creature), Target::Player(ACTIVE)])).unwrap();
            g.run();
            for player in [ACTIVE, FIRST] {
                let request = g.pending().cloned().unwrap();
                assert_eq!(request.player, player);
                assert!(matches!(request.kind, RequestKind::ChooseReplacement { .. }));
                assert_eq!(g.state.player(ACTIVE).poison(), current);
                assert_eq!(g.state.obj(creature).counters(CounterKind::PlusOne), i32::from(current));
                g.answer(request.id, player, Answer::Index(0)).unwrap();
                g.run();
            }
            assert_eq!(g.state.player(ACTIVE).poison(), current + 4);
            assert_eq!(g.state.obj(creature).counters(CounterKind::PlusOne), i32::from(current + 4));
        }
        assert_eq!(g.state.player(FIRST).life, if combat { 19 } else { 20 });
        assert_eq!(g.log.iter().filter(|r| matches!(r.event,
            Event::CountersPut { amount: 4, by: ACTIVE, .. })).count(), usize::from(times));
    }
}

/// CR 616.1, 800.4a: a creature returned by its controller's concession is now affected by its surviving
/// controller's Doubling Seasons, including when its own counter-replacement question was already outstanding.
/// Reachable with Ray of Command taking Vampire Nighthawk, Plague Stinger in combat, and two Seasons on each side.
#[test]
fn counter_result_rechecks_controller_during_its_own_replacement_choice() {
    let mut g = game();
    let seasons = [put(&mut g, &REVIEW_SEASON, ACTIVE), put(&mut g, &REVIEW_SEASON, ACTIVE)];
    put(&mut g, &REVIEW_SEASON, FIRST);
    put(&mut g, &REVIEW_SEASON, FIRST);
    let stinger = put(&mut g, &REVIEW_STINGER, ACTIVE);
    let nighthawk = put(&mut g, &LIFELINKER, ACTIVE);
    let controller_source = put(&mut g, &PYROHEMIA, FIRST);
    cantrip_api::Game::add_effect(&mut cx(&mut g), controller_source, nighthawk, EffectKind::Control(FIRST), Duration::EndOfTurn);
    g.state.set_attackers(vec![
        Attacker { blocked: true, ..Attacker::new(stinger, Target::Player(FIRST), FIRST, TypeSet::NONE) },
    ]);
    g.state.set_blocks(vec![Block { blocker: nighthawk, attacker: stinger, first_strike: false }]);
    g.state.update_turn(|t| t.set_step(Step::CombatDamage));
    g.state.frame_push(Frame::new(FrameKind::BeginStep));
    g.run();
    assert_eq!(g.pending().unwrap().player, FIRST);
    assert!(matches!(g.pending().unwrap().kind, RequestKind::ChooseReplacement { subject: Some(id), .. } if id == nighthawk));
    g.concede(FIRST);
    assert_eq!(g.state.obj(nighthawk).controller, ACTIVE);
    let request = g.pending().cloned().expect("the returned creature's controller orders its two Seasons");
    assert!(matches!(&request.kind, RequestKind::ChooseReplacement { subject: Some(id), sources, .. }
        if *id == nighthawk && *sources == seasons), "expected returned recipient's replacements, got {:?}; counters={}",
        request.kind, g.state.obj(nighthawk).counters(CounterKind::MinusOne));
    assert_eq!(request.player, ACTIVE);
    g.answer(request.id, ACTIVE, Answer::Index(0)).unwrap();
    g.run();
    assert!(g.log.iter().any(|r| matches!(r.event,
        Event::CountersPut { object, kind: CounterKind::MinusOne, amount: 4, by: ACTIVE, .. } if object == nighthawk)));
}

/// The same outstanding-choice concession now returns the creature to Melira's protection. No replacement of the
/// prohibited placement may be offered or applied (CR 101.2, 611.3a), even though damage itself was already dealt.
#[test]
fn counter_result_rechecks_prohibition_during_its_own_replacement_choice() {
    let mut g = game();
    put(&mut g, &MELIRA, ACTIVE);
    put(&mut g, &REVIEW_SEASON, ACTIVE);
    put(&mut g, &REVIEW_SEASON, ACTIVE);
    put(&mut g, &REVIEW_SEASON, FIRST);
    put(&mut g, &REVIEW_SEASON, FIRST);
    let stinger = put(&mut g, &REVIEW_STINGER, ACTIVE);
    let nighthawk = put(&mut g, &LIFELINKER, ACTIVE);
    let controller_source = put(&mut g, &PYROHEMIA, FIRST);
    cantrip_api::Game::add_effect(&mut cx(&mut g), controller_source, nighthawk, EffectKind::Control(FIRST), Duration::EndOfTurn);
    g.state.set_attackers(vec![Attacker { blocked: true,
        ..Attacker::new(stinger, Target::Player(FIRST), FIRST, TypeSet::NONE) }]);
    g.state.set_blocks(vec![Block { blocker: nighthawk, attacker: stinger, first_strike: false }]);
    g.state.update_turn(|t| t.set_step(Step::CombatDamage));
    g.state.frame_push(Frame::new(FrameKind::BeginStep));
    g.run();
    assert_eq!(g.pending().unwrap().player, FIRST);
    assert!(matches!(g.pending().unwrap().kind, RequestKind::ChooseReplacement { .. }));
    g.concede(FIRST);
    assert_eq!(g.state.obj(nighthawk).controller, ACTIVE);
    assert!(!matches!(g.pending().map(|r| &r.kind), Some(RequestKind::ChooseReplacement { .. })));
    assert_eq!(g.state.obj(nighthawk).counters(CounterKind::MinusOne), 0);
    assert!(!g.log.iter().any(|r| matches!(r.event, Event::CountersPut { kind: CounterKind::MinusOne, .. })));
    assert_eq!(g.log.iter().filter(|r| matches!(r.event, Event::Damage { .. })).count(), 2);
}

/// CR 614.12: entry counters use the prospective battlefield controller, not the object's controller/owner in its
/// previous zone. Doubling Season and Melira must both judge the proposed entry, including the post-replacement check.
#[test]
fn entering_counters_keep_the_prospective_controller_for_choices_and_prohibitions() {
    for zone in [Zone::graveyard(YOU), Zone::STACK] {
        for banned in [false, true] {
            let mut g = game();
            let seasons = [put(&mut g, &REVIEW_SEASON, ACTIVE), put(&mut g, &REVIEW_SEASON, ACTIVE)];
            put(&mut g, &MELIRA, if banned { ACTIVE } else { YOU });
            let card = g.cards.id_of(&LIFELINKER);
            let object = g.state.push_obj(Obj::new(card, YOU, zone));
            g.state.zone_push(zone, object);
            let mut counts = [0; CounterKind::COUNT];
            counts[CounterKind::MinusOne as usize] = 1;
            let counts = crate::counters::entering(&mut cx(&mut g), object, ACTIVE, LAST, counts);
            assert_eq!(counts[CounterKind::MinusOne as usize], if banned { 0 } else { 4 });
            assert_eq!(g.state.obj(object).controller, YOU, "preparing an entry does not change its old-zone object");
            assert_eq!(g.state.obj(object).counters(CounterKind::MinusOne), 0);
            if banned {
                assert!(g.pending().is_none(), "a prohibited entry counter offers no replacement order");
            } else {
                let request = g.pending().unwrap();
                assert_eq!(request.player, ACTIVE);
                assert!(matches!(&request.kind, RequestKind::ChooseReplacement { subject: Some(id), sources, .. }
                    if *id == object && *sources == seasons));
            }
        }
    }
}

#[test]
fn combat_step_uses_one_life_batch_for_multiple_opponents_and_lifelink_sources() {
    let mut g = game();
    let watcher = put(&mut g, &WATCHER, ACTIVE);
    let pridemate = put(&mut g, &PRIDEMATE, ACTIVE);
    let mut attackers = Vec::new();
    for player in [YOU, FIRST, LAST] {
        let hawk = put(&mut g, &HAWK, ACTIVE);
        attackers.push(Attacker::new(hawk, Target::Player(player), player, TypeSet::default()));
    }
    g.state.set_attackers(attackers);
    g.state.update_turn(|t| t.set_step(Step::CombatDamage));
    crate::turn::begin_step(&mut cx(&mut g)).unwrap();
    assert_eq!(life(&g), vec![19, 19, 23, 19]);
    assert_eq!(trigger_count(&g, watcher, 0), 1);
    assert_eq!(trigger_count(&g, watcher, 1), 3);
    assert_eq!(trigger_count(&g, pridemate, 0), 3);
    assert_eq!(g.log.iter().filter(|r| matches!(r.event, Event::Damage { kind, .. } if kind.is_combat())).count(), 3);
}

#[test]
fn sign_changed_lifelink_and_damage_losses_are_announced_together() {
    let mut g = game();
    let watcher = put(&mut g, &WATCHER, YOU);
    let pridemate = put(&mut g, &PRIDEMATE, FIRST);
    put(&mut g, &REMEDY, YOU);
    let source = put(&mut g, &LIFELINKER, FIRST);
    cantrip_api::Game::damage(&mut cx(&mut g), source, Target::Player(LAST), 1);
    assert_eq!(changes(&g), vec![(LAST, -1), (FIRST, -1)]);
    assert_eq!(trigger_count(&g, watcher, 0), 1, "Tainted Remedy's result belongs to the same damage event");
    assert_eq!(trigger_count(&g, watcher, 1), 2);
    assert_eq!(trigger_count(&g, pridemate, 0), 0);
}

#[test]
fn prohibited_life_results_do_not_replace_or_prevent_the_damage_itself() {
    for block_gain in [false, true] {
        let mut g = game();
        put(&mut g, &EMPERION, FIRST);
        put(&mut g, &DOUBLER, ACTIVE);
        put(&mut g, &DOUBLER, ACTIVE);
        put(&mut g, &ANGEL, ACTIVE);
        let source = put(&mut g, &LIFELINKER, ACTIVE);
        if block_gain {
            put(&mut g, &REMEDY, YOU);
            g.state.effect_push(ContinuousEffect {
                source, affected: source, controller: ACTIVE, timestamp: 1, words: Words::NONE,
                kind: EffectKind::CantGainLife(ACTIVE), duration: Duration::EndOfTurn,
            });
        }
        assert_eq!(cantrip_api::Game::damage(&mut cx(&mut g), source, Target::Player(FIRST), 2), 2);
        assert_eq!(g.state.player(FIRST).life, 20);
        assert_eq!(changes(&g), if block_gain { vec![] } else { vec![(ACTIVE, 3)] });
        assert!(g.state.agenda[0].pending.is_none(), "prohibited results offer no replacement choices");
        assert_eq!(g.log.iter().filter(|r| matches!(r.event, Event::Damage { amount: 2, .. })).count(), 1);
    }
}

#[test]
fn damage_life_results_use_only_the_damage_left_after_prevention() {
    let mut g = game();
    g.state.update_turn(|t| t.number = 1);
    let watcher = put(&mut g, &WATCHER, YOU);
    put(&mut g, &ARMOR, FIRST);
    put(&mut g, &ANGEL, ACTIVE);
    let one = put(&mut g, &LIFELINKER, ACTIVE);
    let two = put(&mut g, &LIFELINKER, ACTIVE);
    assert_eq!(cantrip_api::Game::damage_together(&mut cx(&mut g), &[
        (one, Target::Player(FIRST), 1), (two, Target::Player(FIRST), 2),
    ]), vec![0, 1]);
    assert_eq!(changes(&g), vec![(ACTIVE, 2), (FIRST, -1)]);
    assert_eq!(trigger_count(&g, watcher, 0), 1);
    assert_eq!(g.log.iter().filter(|r| matches!(r.event, Event::DamagePrevented { amount: 1, .. })).count(), 2);
}

#[test]
fn combat_infect_and_toxic_keep_their_poison_results_alongside_life_and_lifelink() {
    let mut g = game();
    let watcher = put(&mut g, &WATCHER, YOU);
    let mut attackers = Vec::new();
    for def in [&INFECT, &TOXIC, &HAWK] {
        let source = put(&mut g, def, ACTIVE);
        attackers.push(Attacker::new(source, Target::Player(FIRST), FIRST, TypeSet::default()));
    }
    g.state.set_attackers(attackers);
    g.state.update_turn(|t| t.set_step(Step::CombatDamage));
    crate::turn::begin_step(&mut cx(&mut g)).unwrap();
    assert_eq!(changes(&g), vec![(ACTIVE, 1), (FIRST, -2)]);
    assert_eq!(g.state.player(FIRST).poison(), 2, "one infect counter and one toxic counter");
    assert_eq!(trigger_count(&g, watcher, 0), 0, "only noninfect damage contributes to the combined loss");
    assert_eq!(trigger_count(&g, watcher, 1), 1);
}

#[test]
fn simultaneous_damage_life_results_keep_large_totals_at_full_width() {
    let mut g = game();
    let source = put(&mut g, &LIFELINKER, ACTIVE);
    assert_eq!(cantrip_api::Game::damage_together(&mut cx(&mut g), &[
        (source, Target::Player(FIRST), 40_000), (source, Target::Player(FIRST), 40_000),
    ]), vec![40_000, 40_000]);
    assert_eq!(changes(&g), vec![(ACTIVE, 80_000), (FIRST, -80_000)]);
    assert_eq!(g.state.player(FIRST).life, -79_980);
    assert_eq!(g.state.player(ACTIVE).life, 80_020);
}

#[test]
fn different_losses_are_one_batch_but_individual_occurrences() {
    let mut g = game();
    let watcher = put(&mut g, &WATCHER, YOU);
    let lost = cantrip_api::Game::lose_life_together(&mut cx(&mut g), &[(FIRST, 1), (ACTIVE, 3), (LAST, 1)]);
    assert_eq!(lost, 5);
    assert_eq!(life(&g), vec![20, 19, 17, 19]);
    assert_eq!(changes(&g), vec![(ACTIVE, -3), (LAST, -1), (FIRST, -1)]);
    assert_eq!(trigger_count(&g, watcher, 0), 1, "Ob Nixilis sees one occurrence, not two");
    assert_eq!(trigger_count(&g, watcher, 1), 3, "Mindcrank sees each opponent's loss");
}

#[test]
fn each_opponent_uses_the_same_simultaneous_operation() {
    let mut g = game();
    let watcher = put(&mut g, &WATCHER, ACTIVE);
    assert_eq!(cantrip_api::Game::each_opponent_loses_life(&mut cx(&mut g), ACTIVE, 1), 3);
    assert_eq!(changes(&g), vec![(LAST, -1), (YOU, -1), (FIRST, -1)]);
    assert_eq!(trigger_count(&g, watcher, 0), 1);
    assert_eq!(trigger_count(&g, watcher, 1), 3);
}

#[test]
fn a_players_entries_are_one_loss_replaced_and_announced_once() {
    let mut g = game();
    put(&mut g, &DOUBLER, ACTIVE);
    let watcher = put(&mut g, &WATCHER, ACTIVE);
    assert_eq!(cantrip_api::Game::lose_life_together(&mut cx(&mut g), &[(FIRST, 1), (YOU, 3), (FIRST, 2)]), 12);
    assert_eq!(changes(&g), vec![(YOU, -6), (FIRST, -6)]);
    assert_eq!(life(&g), vec![14, 14, 20, 20]);
    assert_eq!(trigger_count(&g, watcher, 1), 2);
}

#[test]
fn blocked_losses_are_not_replaced_or_counted() {
    let mut g = game();
    put(&mut g, &EMPERION, FIRST);
    let source = put(&mut g, &DOUBLER, ACTIVE);
    put(&mut g, &DOUBLER, ACTIVE);
    g.state.effect_push(ContinuousEffect {
        source, affected: source, controller: LAST, timestamp: 1, words: Words::NONE,
        kind: EffectKind::LifeTotalCantChange(LAST), duration: Duration::EndOfTurn,
    });
    assert_eq!(cantrip_api::Game::lose_life_together(&mut cx(&mut g), &[(FIRST, 2), (LAST, 7)]), 0);
    assert_eq!(life(&g), vec![20; 4]);
    assert!(changes(&g).is_empty());
    assert!(g.state.agenda[0].pending.is_none(), "no replacement choices for prohibited losses");
    assert_eq!(cantrip_api::Game::lose_life_together(&mut cx(&mut g), &[(FIRST, 2), (ACTIVE, 4), (LAST, 7)]), 4);
    assert_eq!(changes(&g), vec![(ACTIVE, -4)]);
}

#[test]
fn nonpositive_amounts_and_departed_players_do_nothing() {
    let mut g = game();
    g.state.update_player(FIRST, |p| p.lost = true);
    put(&mut g, &DOUBLER, ACTIVE);
    put(&mut g, &DOUBLER, ACTIVE);
    let mut c = cx(&mut g);
    assert_eq!(cantrip_api::Game::lose_life_together(&mut c, &[]), 0);
    assert_eq!(cantrip_api::Game::lose_life_together(&mut c, &[(FIRST, 8), (YOU, 0), (LAST, -2)]), 0);
    assert_eq!(cantrip_api::Game::each_opponent_loses_life(&mut c, ACTIVE, 0), 0);
    assert_eq!(life(&g), vec![20; 4]);
    assert!(g.log.is_empty());
    assert!(g.state.agenda[0].pending.is_none());
}

#[test]
fn live_players_can_lose_below_zero_and_large_amounts_are_kept_whole() {
    let mut g = game();
    g.state.update_player(FIRST, |p| p.life = 0);
    assert_eq!(cantrip_api::Game::lose_life_together(&mut cx(&mut g), &[(FIRST, 70_000), (LAST, 80_000)]), 150_000);
    assert_eq!(life(&g), vec![20, -70_000, 20, -79_980]);
    assert!(g.state.in_game(FIRST), "losing life does not run state-based actions mid-instruction");
}

#[test]
fn single_player_changes_remain_individual_and_keep_sign_changing_replacements() {
    let mut g = game();
    let watcher = put(&mut g, &WATCHER, YOU);
    cantrip_api::Game::lose_life(&mut cx(&mut g), FIRST, 1);
    cantrip_api::Game::lose_life(&mut cx(&mut g), LAST, 1);
    assert_eq!(trigger_count(&g, watcher, 0), 2, "separate calls remain separate events");
    put(&mut g, &REMEDY, YOU);
    cantrip_api::Game::gain_life(&mut cx(&mut g), FIRST, 3);
    cantrip_api::Game::set_life_total(&mut cx(&mut g), LAST, 22);
    assert_eq!(life(&g), vec![20, 16, 20, 16]);
    assert_eq!(changes(&g), vec![(FIRST, -1), (LAST, -1), (FIRST, -3), (LAST, -3)]);
    put(&mut g, &EMPERION, FIRST);
    cantrip_api::Game::gain_life(&mut cx(&mut g), FIRST, 3);
    cantrip_api::Game::lose_life(&mut cx(&mut g), FIRST, 3);
    assert_eq!(g.state.player(FIRST).life, 16);
    assert_eq!(changes(&g).len(), 4);
}

#[test]
fn concession_removes_replacements_before_the_question_is_rebuilt() {
    let mut g = game();
    put(&mut g, &DOUBLER, ACTIVE);
    put(&mut g, &DOUBLER, ACTIVE);
    let card = g.cards.id_of(&DRAIN);
    let source = g.state.push_obj(Obj::new(card, YOU, Zone::STACK));
    g.state.zone_push(Zone::STACK, source);
    g.state.stack_push(crate::actions::stack_item(StackKind::Spell, source, AbilityRef::spell(card), YOU, &[]));
    let serial = g.state.stack.last().unwrap().serial;
    g.state.frame_push(Frame::new(FrameKind::Resolve { serial }));
    g.run();
    assert_eq!(g.pending().unwrap().player, LAST);
    assert!(matches!(g.pending().unwrap().kind, RequestKind::ChooseReplacement { .. }));

    g.concede(ACTIVE);
    assert!(!matches!(g.pending().map(|r| &r.kind), Some(RequestKind::ChooseReplacement { .. })), "removed Bloodletters must not be offered");
    assert_eq!(changes(&g), vec![(LAST, -1), (FIRST, -1), (YOU, 2)]);
    assert_eq!(life(&g), vec![22, 19, 20, 19]);
}

/// Real replacement questions and resolution replay, not preloaded answers: every opponent orders two Bloodletters
/// before Exsanguinate changes any total. Undo/redo restores both the pre-instruction request and the final history.
#[test]
fn replacement_choices_are_apnap_before_all_updates_and_survive_undo_redo() {
    let mut g = game();
    let one = put(&mut g, &DOUBLER, ACTIVE);
    let two = put(&mut g, &DOUBLER, ACTIVE);
    let card = g.cards.id_of(&DRAIN);
    let source = g.state.push_obj(Obj::new(card, ACTIVE, Zone::STACK));
    g.state.zone_push(Zone::STACK, source);
    g.state.stack_push(crate::actions::stack_item(StackKind::Spell, source, AbilityRef::spell(card), ACTIVE, &[]));
    let serial = g.state.stack.last().unwrap().serial;
    g.state.frame_push(Frame::new(FrameKind::Resolve { serial }));
    g.run();
    let before = g.at;
    for player in [LAST, YOU, FIRST] {
        let request = g.pending().cloned().expect("replacement question");
        assert_eq!(request.player, player, "APNAP, not seat order or the loss input order");
        let RequestKind::ChooseReplacement { life: amount, sources, .. } = &request.kind else { panic!("{:?}", request.kind) };
        assert_eq!(*amount, Some(-1));
        assert_eq!(*sources, vec![one, two]);
        assert_eq!(life(&g), vec![20; 4], "every choice precedes the original updates");
        assert!(changes(&g).is_empty());
        g.answer(request.id, player, Answer::Index(1)).expect("valid order");
        g.run();
    }
    let after = g.at;
    let expected = vec![(LAST, -4), (YOU, -4), (FIRST, -4), (ACTIVE, 12)];
    assert_eq!(changes(&g), expected);
    assert_eq!(life(&g), vec![16, 16, 32, 16]);
    g.goto(before);
    assert_eq!(g.pending().unwrap().player, LAST);
    assert_eq!(life(&g), vec![20; 4]);
    assert!(changes(&g).is_empty());
    g.goto(after);
    assert_eq!(changes(&g), expected);
    assert_eq!(life(&g), vec![16, 16, 32, 16]);
}

fn life_after_land_destroyed(q: &dyn Query, _: &Asker, event: &Event) -> Option<Fired> {
    if matches!(event, Event::LifeChange { player: YOU, delta: -1 }) {
        assert!(!q.state().battlefield().iter().any(|&id| q.chars(id).is(CardType::Land)), "the life payment and destruction must already both have happened");
        Fired::PLAIN
    } else { None }
}

static LAND_PAYMENT_OBSERVER: CardDef = CardDef {
    name: "Simultaneous life and land observer",
    types: TypeSet::of(&[CardType::Enchantment]),
    abilities: &[triggered(when_asked(EventMask::LIFE_CHANGE, life_after_land_destroyed), nothing, "Whenever you lose life, observe the battlefield.")],
    ..CardDef::BLANK
};
static LAND_PAYMENT_LAND: CardDef = CardDef { name: "Simultaneous life and land", types: TypeSet::of(&[CardType::Land]), ..CardDef::BLANK };

#[test]
fn life_cost_and_destroy_are_one_instruction() {
    use cantrip_api::Game as _;
    let table = Box::leak(Box::new(CardTable::new([&LAND_PAYMENT_OBSERVER, &LAND_PAYMENT_LAND])));
    let mut g = Game::new(table, &Setup { decks: vec![Vec::new(); 2], ..Default::default() });
    let watcher = g.state.push_obj(Obj::new(table.id_of(&LAND_PAYMENT_OBSERVER), YOU, Zone::BATTLEFIELD));
    g.state.zone_push(Zone::BATTLEFIELD, watcher);
    let land = g.state.push_obj(Obj::new(table.id_of(&LAND_PAYMENT_LAND), YOU, Zone::BATTLEFIELD));
    g.state.zone_push(Zone::BATTLEFIELD, land);
    g.state.see_statics();
    let mut c = cx(&mut g);
    assert!(c.pay_life_with_permanents(land, &[(YOU, 1)], &[land], &[]));
    assert_eq!(c.state().player(YOU).life, 19);
}

#[test]
fn simultaneous_life_costs_check_total_affordability_before_any_action() {
    use cantrip_api::Game as _;
    let table = Box::leak(Box::new(CardTable::new([&LAND_PAYMENT_LAND])));
    let mut g = Game::new(table, &Setup { decks: vec![Vec::new(); 2], ..Default::default() });
    let land = g.state.push_obj(Obj::new(table.id_of(&LAND_PAYMENT_LAND), YOU, Zone::BATTLEFIELD));
    g.state.zone_push(Zone::BATTLEFIELD, land);
    g.state.update_player(YOU, |p| p.life = 1);
    let mut c = cx(&mut g);
    assert!(!c.pay_life_with_permanents(land, &[(YOU, 1), (YOU, 1)], &[land], &[]));
    assert_eq!(c.state().player(YOU).life, 1);
    assert_eq!(c.obj(land).zone, Zone::BATTLEFIELD);
    assert!(!c.g.log.iter().any(|r| matches!(r.event, Event::ResolutionPaymentBegan { .. })));
    assert!(c.pay_life_with_permanents(land, &[(YOU, 1)], &[land], &[]));
    assert_eq!(c.state().player(YOU).life, 0);
    assert!(c.obj(land).became.is_some());
    assert_eq!(c.g.log.iter().filter(|r| matches!(r.event, Event::ResolutionPaymentBegan { payer, source, .. } if payer == YOU && source == land)).count(), 1);
}

fn land_to_graveyard(_: &dyn Query, _: &Asker, proposal: &Proposal) -> bool {
    matches!(proposal, Proposal::ZoneChange(z) if z.destruction())
}
fn exile_land_instead(_: &dyn Query, _: &Asker, proposal: &mut Proposal) {
    if let Proposal::ZoneChange(z) = proposal { z.to = Zone::exile(z.to.player); }
}
static LAND_EXILER: CardDef = CardDef {
    name: "Simultaneous replacement Rest in Peace",
    types: TypeSet::of(&[CardType::Enchantment]),
    replacements: &[replacement(ReplacementKind::ZoneChange, land_to_graveyard, exile_land_instead, "Exile the land instead.")],
    ..CardDef::BLANK
};
fn mixed_payment(g: &mut dyn cantrip_api::Game, item: &StackItem) -> Result<(), Suspend> {
    let lands: Vec<_> = g.state().battlefield().iter().copied().filter(|&id| g.chars(id).is(CardType::Land)).collect();
    g.pay_life_with_permanents(item.object, &[(LAST, 1)], &lands, &[]);
    Ok(())
}
static MIXED_PAYMENT: CardDef = CardDef {
    name: "Simultaneous payment and destruction",
    types: TypeSet::of(&[CardType::Sorcery]), spell: spell(&[], mixed_payment), ..CardDef::BLANK
};
/// The payer is earlier in APNAP than the land's controller; both choose between printed replacement effects.
#[test]
fn mixed_life_and_destruction_replacement_choices_follow_apnap() {
    let mut g = game();
    put(&mut g, &DOUBLER, ACTIVE); put(&mut g, &DOUBLER, ACTIVE);
    put(&mut g, &LAND_EXILER, ACTIVE); put(&mut g, &LAND_EXILER, ACTIVE);
    put(&mut g, &LAND_PAYMENT_LAND, FIRST);
    let card = g.cards.id_of(&MIXED_PAYMENT);
    let source = g.state.push_obj(Obj::new(card, ACTIVE, Zone::STACK));
    g.state.zone_push(Zone::STACK, source);
    g.state.stack_push(crate::actions::stack_item(StackKind::Spell, source, AbilityRef::spell(card), ACTIVE, &[]));
    let serial = g.state.stack.last().unwrap().serial;
    g.state.frame_push(Frame::new(FrameKind::Resolve { serial }));
    g.run();
    let before = g.at;
    let request = g.pending().cloned().expect("replacement choice");
    assert_eq!(request.player, LAST, "the earlier affected player chooses life replacements first");
    g.answer(request.id, LAST, Answer::Index(0)).unwrap();
    g.run();
    let request = g.pending().cloned().expect("land replacement choice");
    assert_eq!(request.player, FIRST);
    g.answer(request.id, FIRST, Answer::Index(0)).unwrap();
    g.run();
    let after = g.at;
    assert_eq!(g.state.player(LAST).life, 16);
    assert_eq!(g.state.zone(Zone::exile(FIRST)).len(), 1);
    assert_eq!(g.log.iter().filter(|r| matches!(r.event, Event::ResolutionPaymentBegan {
        source: recorded, payer: LAST, resolving: Some(which), .. } if recorded == source && which == serial)).count(), 1);
    g.goto(before);
    assert_eq!(g.pending().unwrap().player, LAST);
    assert_eq!(g.state.player(LAST).life, 20);
    g.goto(after);
    assert_eq!(g.state.player(LAST).life, 16);
    assert_eq!(g.state.zone(Zone::exile(FIRST)).len(), 1);
}

fn same_payment(g: &mut dyn cantrip_api::Game, item: &StackItem) -> Result<(), Suspend> {
    let lands: Vec<_> = g.state().battlefield().iter().copied().filter(|&id| g.chars(id).is(CardType::Land)).collect();
    g.pay_life_with_permanents(item.object, &[(FIRST, 1)], &lands, &[]);
    Ok(())
}
static SAME_PAYMENT: CardDef = CardDef {
    name: "Same player payment and destruction",
    types: TypeSet::of(&[CardType::Sorcery]), spell: spell(&[], same_payment), ..CardDef::BLANK
};
/// CR 101.4c: the affected player can choose either simultaneous replacement question first.
#[test]
fn mixed_replacement_questions_for_one_player_can_be_ordered() {
    let mut g = game();
    put(&mut g, &DOUBLER, ACTIVE); put(&mut g, &DOUBLER, ACTIVE);
    put(&mut g, &LAND_EXILER, ACTIVE); put(&mut g, &LAND_EXILER, ACTIVE);
    let land = put(&mut g, &LAND_PAYMENT_LAND, FIRST);
    let card = g.cards.id_of(&SAME_PAYMENT);
    let source = g.state.push_obj(Obj::new(card, ACTIVE, Zone::STACK));
    g.state.zone_push(Zone::STACK, source);
    g.state.stack_push(crate::actions::stack_item(StackKind::Spell, source, AbilityRef::spell(card), ACTIVE, &[]));
    let serial = g.state.stack.last().unwrap().serial;
    g.state.frame_push(Frame::new(FrameKind::Resolve { serial }));
    g.run();
    let request = g.pending().cloned().expect("order choice");
    assert_eq!(request.player, FIRST);
    let RequestKind::OrderReplacementOccurrences { occurrences } = &request.kind else { panic!("expected an occurrence-order choice: {:?}", request.kind) };
    let life = occurrences.iter().position(|o| matches!(o, ReplacementOccurrence::Life { player, amount: -1, .. } if *player == FIRST)).unwrap();
    let object = occurrences.iter().position(|o| matches!(o, ReplacementOccurrence::Object { object, .. } if *object == land)).unwrap();
    assert_ne!(life, object);
    g.answer(request.id, FIRST, Answer::CostPayment(object)).unwrap();
    g.run();
    let request = g.pending().cloned().expect("land replacements");
    assert!(matches!(&request.kind, RequestKind::ChooseReplacement { subject: Some(id), .. } if *id == land));
    g.answer(request.id, FIRST, Answer::Index(0)).unwrap();
    g.run();
    let request = g.pending().cloned().expect("life replacements");
    assert!(matches!(&request.kind, RequestKind::ChooseReplacement { life: Some(-1), .. }));
}

const OPTIONAL_EXILE: Question = Question::new("OptionalExile", "exile it instead?", "", "Exile", "Do not exile");
static OPTIONAL_LAND_EXILER: CardDef = CardDef {
    name: "Optional exile the destroyed land", types: TypeSet::of(&[CardType::Enchantment]),
    replacements: &[replacement(ReplacementKind::ZoneChange, land_to_graveyard, exile_land_instead, "You may exile it instead.").optional(&OPTIONAL_EXILE)],
    ..CardDef::BLANK
};

#[test]
fn mixed_event_optional_replacement_asks_its_controller_in_apnap_order_and_replays() {
    let mut g = game();
    put(&mut g, &DOUBLER, ACTIVE); put(&mut g, &DOUBLER, ACTIVE);
    put(&mut g, &OPTIONAL_LAND_EXILER, YOU);
    put(&mut g, &LAND_PAYMENT_LAND, FIRST);
    let card = g.cards.id_of(&MIXED_PAYMENT);
    let source = g.state.push_obj(Obj::new(card, ACTIVE, Zone::STACK));
    g.state.zone_push(Zone::STACK, source);
    g.state.stack_push(crate::actions::stack_item(StackKind::Spell, source, AbilityRef::spell(card), ACTIVE, &[]));
    let serial = g.state.stack.last().unwrap().serial;
    g.state.frame_push(Frame::new(FrameKind::Resolve { serial }));
    g.run();
    let before = g.at;
    let request = g.pending().cloned().expect("life replacements");
    assert_eq!(request.player, LAST);
    g.answer(request.id, LAST, Answer::Index(0)).unwrap();
    g.run();
    let request = g.pending().cloned().expect("optional replacement");
    assert_eq!(request.player, YOU, "the optional effect's controller, not the land's controller");
    assert!(matches!(request.kind, RequestKind::May { .. }));
    g.answer(request.id, YOU, Answer::No).unwrap();
    g.run();
    let after = g.at;
    assert_eq!(g.state.player(LAST).life, 16);
    assert_eq!(g.state.zone(Zone::graveyard(FIRST)).len(), 1);
    g.goto(before);
    assert_eq!(g.pending().unwrap().player, LAST);
    g.goto(after);
    assert_eq!(g.state.player(LAST).life, 16);
    assert_eq!(g.state.zone(Zone::graveyard(FIRST)).len(), 1);
}

#[test]
fn mixed_replacement_order_restarts_when_a_concession_changes_land_controller() {
    let mut g = game();
    put(&mut g, &DOUBLER, ACTIVE); put(&mut g, &DOUBLER, ACTIVE);
    put(&mut g, &LAND_EXILER, ACTIVE); put(&mut g, &LAND_EXILER, ACTIVE);
    let land = put(&mut g, &LAND_PAYMENT_LAND, ACTIVE);
    let control_source = put(&mut g, &DRAIN, FIRST);
    cantrip_api::Game::add_effect(&mut cx(&mut g), control_source, land, EffectKind::Control(FIRST), Duration::EndOfTurn);
    let card = g.cards.id_of(&MIXED_PAYMENT);
    let source = g.state.push_obj(Obj::new(card, ACTIVE, Zone::STACK));
    g.state.zone_push(Zone::STACK, source);
    g.state.stack_push(crate::actions::stack_item(StackKind::Spell, source, AbilityRef::spell(card), ACTIVE, &[]));
    let serial = g.state.stack.last().unwrap().serial;
    g.state.frame_push(Frame::new(FrameKind::Resolve { serial }));
    g.run();
    assert_eq!(g.pending().unwrap().player, LAST);
    g.concede(FIRST);
    let request = g.pending().cloned().expect("returned land replacement choice");
    assert_eq!(request.player, ACTIVE, "the returned land creates an earlier APNAP choice");
    assert!(matches!(&request.kind, RequestKind::ChooseReplacement { subject: Some(id), .. } if *id == land));
}

fn destroyed_card_component(_: &dyn Query, _: &Asker, proposal: &Proposal) -> bool {
    matches!(proposal, Proposal::ZoneChange(z) if z.destruction() && z.is_card)
}
static CARD_COMPONENT_EXILER: CardDef = CardDef {
    name: "Card component exile replacement", types: TypeSet::of(&[CardType::Enchantment]),
    replacements: &[replacement(ReplacementKind::ZoneChange, destroyed_card_component, exile_land_instead, "Exile the card instead.")],
    ..CardDef::BLANK
};

fn merged_token_land(g: &mut Game, controller: PlayerId) -> ObjectId {
    let land = put(g, &LAND_PAYMENT_LAND, controller);
    g.state.update_obj(land, |o| o.token = true);
    let card = g.cards.id_of(&LAND_PAYMENT_LAND);
    g.state.set_merge(land, cantrip_state::Merge {
        own_face: card, own_token: true, own_commander_id: None, own_face_down: None,
        parts: vec![cantrip_state::MergedPart { card, face: card, token: false, commander_id: None, copy: None,
            on_top: false, timestamp: 0, prior: None, melded: false }],
    });
    land
}

/// CR 730.3e: a token's card-only component replacements join the same APNAP scheduling, after its parent move survives.
#[test]
fn mixed_merged_token_card_component_replacements_follow_apnap() {
    let mut g = game();
    put(&mut g, &DOUBLER, ACTIVE); put(&mut g, &DOUBLER, ACTIVE);
    put(&mut g, &CARD_COMPONENT_EXILER, ACTIVE); put(&mut g, &CARD_COMPONENT_EXILER, ACTIVE);
    let land = merged_token_land(&mut g, ACTIVE);
    let card = g.cards.id_of(&MIXED_PAYMENT);
    let source = g.state.push_obj(Obj::new(card, ACTIVE, Zone::STACK));
    g.state.zone_push(Zone::STACK, source);
    g.state.stack_push(crate::actions::stack_item(StackKind::Spell, source, AbilityRef::spell(card), ACTIVE, &[]));
    let serial = g.state.stack.last().unwrap().serial;
    g.state.frame_push(Frame::new(FrameKind::Resolve { serial }));
    g.run();
    let request = g.pending().cloned().expect("card component replacement choice");
    assert_eq!(request.player, ACTIVE, "the card component's controller chooses before the life payer");
    assert!(matches!(&request.kind, RequestKind::ChooseReplacement { subject: Some(id), .. } if *id == land));
    g.answer(request.id, ACTIVE, Answer::Index(0)).unwrap();
    g.run();
    assert_eq!(g.pending().unwrap().player, LAST);
}

#[test]
fn mixed_merged_token_regeneration_never_asks_card_component_replacements() {
    let mut g = game();
    put(&mut g, &DOUBLER, ACTIVE); put(&mut g, &DOUBLER, ACTIVE);
    put(&mut g, &CARD_COMPONENT_EXILER, ACTIVE); put(&mut g, &CARD_COMPONENT_EXILER, ACTIVE);
    let land = merged_token_land(&mut g, ACTIVE);
    cantrip_api::Game::add_effect(&mut cx(&mut g), land, land, EffectKind::RegenerationShield, Duration::EndOfTurn);
    let card = g.cards.id_of(&MIXED_PAYMENT);
    let source = g.state.push_obj(Obj::new(card, ACTIVE, Zone::STACK));
    g.state.zone_push(Zone::STACK, source);
    g.state.stack_push(crate::actions::stack_item(StackKind::Spell, source, AbilityRef::spell(card), ACTIVE, &[]));
    let serial = g.state.stack.last().unwrap().serial;
    g.state.frame_push(Frame::new(FrameKind::Resolve { serial }));
    g.run();
    let request = g.pending().cloned().expect("life replacement choice");
    assert_eq!(request.player, LAST, "no choice for a component whose parent regenerates");
    g.answer(request.id, LAST, Answer::Index(0)).unwrap();
    g.run();
    assert_eq!(g.state.obj(land).zone, Zone::BATTLEFIELD);
    assert!(g.state.zone(Zone::exile(ACTIVE)).is_empty());
}

#[test]
fn mixed_replacement_choices_can_alternate_between_one_players_events() {
    let mut g = game();
    for _ in 0..3 { put(&mut g, &DOUBLER, ACTIVE); put(&mut g, &LAND_EXILER, ACTIVE); }
    let land = put(&mut g, &LAND_PAYMENT_LAND, FIRST);
    let card = g.cards.id_of(&SAME_PAYMENT);
    let source = g.state.push_obj(Obj::new(card, ACTIVE, Zone::STACK));
    g.state.zone_push(Zone::STACK, source);
    g.state.stack_push(crate::actions::stack_item(StackKind::Spell, source, AbilityRef::spell(card), ACTIVE, &[]));
    let serial = g.state.stack.last().unwrap().serial;
    g.state.frame_push(Frame::new(FrameKind::Resolve { serial }));
    g.run();
    let first = g.pending().cloned().unwrap();
    let RequestKind::OrderReplacementOccurrences { occurrences } = &first.kind else { panic!("first occurrence order") };
    let life = occurrences.iter().position(|o| matches!(o, ReplacementOccurrence::Life { player, amount: -1, .. } if *player == FIRST)).unwrap();
    g.answer(first.id, FIRST, Answer::CostPayment(life)).unwrap(); g.run();
    let choice = g.pending().cloned().unwrap();
    assert!(matches!(choice.kind, RequestKind::ChooseReplacement { life: Some(-1), .. }));
    g.answer(choice.id, FIRST, Answer::Index(0)).unwrap(); g.run();
    let second = g.pending().cloned().unwrap();
    let RequestKind::OrderReplacementOccurrences { occurrences } = &second.kind else { panic!("order after first life replacement") };
    assert!(occurrences.iter().any(|o| matches!(o, ReplacementOccurrence::Life { amount: -2, .. })));
    let object = occurrences.iter().position(|o| matches!(o, ReplacementOccurrence::Object { object, .. } if *object == land)).unwrap();
    g.answer(second.id, FIRST, Answer::CostPayment(object)).unwrap(); g.run();
    let choice = g.pending().cloned().unwrap();
    assert!(matches!(choice.kind, RequestKind::ChooseReplacement { subject: Some(id), .. } if id == land));
    g.answer(choice.id, FIRST, Answer::Index(0)).unwrap(); g.run();
    let third = g.pending().cloned().unwrap();
    assert!(matches!(third.kind, RequestKind::OrderReplacementOccurrences { .. }), "the player may switch between events again");
}

fn take_land_after_replacement(g: &mut dyn cantrip_api::Game, asker: &Asker, _: &Proposal, _: &Proposal) {
    if let Some(land) = g.state().battlefield().iter().copied().find(|&id| g.chars(id).is(CardType::Land)) {
        g.add_effect(asker.source, land, EffectKind::Control(asker.controller), Duration::EndOfTurn);
    }
}
static CONTROL_ON_LOSS: CardDef = CardDef {
    name: "Replacement continuation changes control", types: TypeSet::of(&[CardType::Enchantment]),
    replacements: &[replacement(ReplacementKind::LifeChange, opponent_loses_on_your_turn, twice,
        "An opponent loses twice that life; then gain control of the land.").then(take_land_after_replacement)],
    ..CardDef::BLANK
};

#[test]
fn mixed_replacement_continuation_restarts_apnap_for_a_new_controller() {
    let mut g = game();
    let changing = put(&mut g, &CONTROL_ON_LOSS, ACTIVE);
    put(&mut g, &DOUBLER, ACTIVE);
    put(&mut g, &LAND_EXILER, ACTIVE); put(&mut g, &LAND_EXILER, ACTIVE);
    let land = put(&mut g, &LAND_PAYMENT_LAND, FIRST);
    let card = g.cards.id_of(&MIXED_PAYMENT);
    let source = g.state.push_obj(Obj::new(card, ACTIVE, Zone::STACK));
    g.state.zone_push(Zone::STACK, source);
    g.state.stack_push(crate::actions::stack_item(StackKind::Spell, source, AbilityRef::spell(card), ACTIVE, &[]));
    let serial = g.state.stack.last().unwrap().serial;
    g.state.frame_push(Frame::new(FrameKind::Resolve { serial }));
    g.run();
    let request = g.pending().cloned().expect("life replacements");
    assert_eq!(request.player, LAST);
    let RequestKind::ChooseReplacement { sources, .. } = &request.kind else { panic!("life replacements") };
    let index = sources.iter().position(|&id| id == changing).unwrap();
    g.answer(request.id, LAST, Answer::Index(index as u8)).unwrap(); g.run();
    let request = g.pending().cloned().expect("new earlier controller's replacement choice");
    assert_eq!(request.player, ACTIVE);
    assert!(matches!(&request.kind, RequestKind::ChooseReplacement { subject: Some(id), .. } if *id == land));
}

fn create_exilers_after_replacement(g: &mut dyn cantrip_api::Game, asker: &Asker, _: &Proposal, _: &Proposal) {
    let card = g.card_id(&LAND_EXILER_WATCHER);
    let _ = g.create_tokens(card, asker.controller, 2, false);
}
static CREATE_EXILERS_ON_LOSS: CardDef = CardDef {
    name: "Replacement continuation creates replacement effects", types: TypeSet::of(&[CardType::Enchantment]),
    replacements: &[replacement(ReplacementKind::LifeChange, opponent_loses_on_your_turn, twice,
        "An opponent loses twice that life; then create two replacement sources.").then(create_exilers_after_replacement)],
    ..CardDef::BLANK
};

#[test]
fn mixed_replacement_continuation_rechecks_previously_unaffected_event() {
    let mut g = game();
    let creating = put(&mut g, &CREATE_EXILERS_ON_LOSS, ACTIVE);
    put(&mut g, &DOUBLER, ACTIVE);
    let land = put(&mut g, &LAND_PAYMENT_LAND, FIRST);
    let card = g.cards.id_of(&MIXED_PAYMENT);
    let source = g.state.push_obj(Obj::new(card, ACTIVE, Zone::STACK));
    g.state.zone_push(Zone::STACK, source);
    g.state.stack_push(crate::actions::stack_item(StackKind::Spell, source, AbilityRef::spell(card), ACTIVE, &[]));
    let serial = g.state.stack.last().unwrap().serial;
    g.state.frame_push(Frame::new(FrameKind::Resolve { serial }));
    g.run();
    let request = g.pending().cloned().expect("life replacements");
    assert_eq!(request.player, LAST);
    let RequestKind::ChooseReplacement { sources, .. } = &request.kind else { panic!("life replacements") };
    let index = sources.iter().position(|&id| id == creating).unwrap();
    g.answer(request.id, LAST, Answer::Index(index as u8)).unwrap(); g.run();
    let request = g.pending().cloned().expect("newly applicable land replacement choice");
    assert_eq!(request.player, FIRST);
    assert!(matches!(&request.kind, RequestKind::ChooseReplacement { subject: Some(id), .. } if *id == land));
    g.answer(request.id, FIRST, Answer::Index(0)).unwrap(); g.run();
    let watcher_card = g.cards.id_of(&LAND_EXILER_WATCHER);
    let watchers: Vec<_> = g.state.battlefield().iter().copied().filter(|&id| g.state.obj(id).card == watcher_card).collect();
    assert_eq!(watchers.len(), 2);
    let order = g.pending().cloned().expect("the two departure triggers must be ordered");
    assert!(matches!(&order.kind, RequestKind::OrderTriggers { sources, .. }
        if sources.len() == 2 && watchers.iter().all(|id| sources.contains(id))),
        "replacement-created permanents see the later simultaneous land departure");
    g.answer(order.id, ACTIVE, Answer::Order(vec![0, 1])).unwrap(); g.run();
    assert_eq!(watchers.iter().map(|&id| waiting_or_stacked(&g, id, ACTIVE)).sum::<usize>(), 2);
}

fn another_permanent_leaves(_: &dyn Query, source: ObjectId, event: &Event) -> Option<Fired> {
    match *event {
        Event::ZoneChange { old, from, .. } if old != source && from.is(ZoneKind::Battlefield) => Fired::PLAIN,
        _ => None,
    }
}
static LAND_EXILER_WATCHER: CardDef = CardDef {
    name: "New replacement source watches departures", types: TypeSet::of(&[CardType::Enchantment]),
    replacements: &[replacement(ReplacementKind::ZoneChange, land_to_graveyard, exile_land_instead, "Exile it instead.")],
    abilities: &[triggered(when(EventMask::ZONE_CHANGE, another_permanent_leaves), nothing, "Whenever another permanent leaves, observe it.")],
    ..CardDef::BLANK
};

#[test]
fn staged_life_payment_respects_a_fixed_life_total() {
    use cantrip_api::Game as _;
    let mut g = game();
    put(&mut g, &EMPERION, YOU);
    let land = put(&mut g, &LAND_PAYMENT_LAND, FIRST);
    let mut c = cx(&mut g);
    assert_eq!(c.payable_resolution_life(YOU), 0);
    assert!(!c.pay_life_with_permanents(land, &[(YOU, 1)], &[land], &[]));
    assert_eq!(c.obj(land).zone, Zone::BATTLEFIELD);
    assert!(!c.g.log.iter().any(|r| matches!(r.event, Event::ResolutionPaymentBegan { .. })));
}

fn two_payments(g: &mut dyn cantrip_api::Game, item: &StackItem) -> Result<(), Suspend> {
    let lands: Vec<_> = g.state().battlefield().iter().copied().filter(|&id| g.chars(id).is(CardType::Land)).collect();
    g.pay_life_with_permanents(item.object, &[(FIRST, 1), (FIRST, 1)], &lands, &[]);
    Ok(())
}
static TWO_PAYMENTS: CardDef = CardDef {
    name: "Two simultaneous life-cost occurrences", types: TypeSet::of(&[CardType::Sorcery]),
    spell: spell(&[], two_payments), ..CardDef::BLANK
};
#[test]
fn mixed_identical_life_occurrences_have_distinct_stable_ordering_identities() {
    let mut g = game();
    for _ in 0..3 { put(&mut g, &DOUBLER, ACTIVE); }
    put(&mut g, &LAND_PAYMENT_LAND, ACTIVE);
    let card = g.cards.id_of(&TWO_PAYMENTS);
    let source = g.state.push_obj(Obj::new(card, ACTIVE, Zone::STACK));
    g.state.zone_push(Zone::STACK, source);
    g.state.stack_push(crate::actions::stack_item(StackKind::Spell, source, AbilityRef::spell(card), ACTIVE, &[]));
    let serial = g.state.stack.last().unwrap().serial;
    g.state.frame_push(Frame::new(FrameKind::Resolve { serial }));
    g.run();
    let request = g.pending().cloned().expect("choose an occurrence");
    let RequestKind::OrderReplacementOccurrences { occurrences } = &request.kind else { panic!("occurrence order") };
    let ids: Vec<_> = occurrences.iter().filter_map(|o| match o {
        ReplacementOccurrence::Life { player, amount: -1, occurrence } if *player == FIRST => Some(*occurrence),
        _ => None,
    }).collect();
    assert_eq!(ids.len(), 2);
    assert_ne!(ids[0], ids[1], "two equal amounts remain distinguishable");
    g.answer(request.id, FIRST, Answer::CostPayment(1)).unwrap(); g.run();
    let choice = g.pending().cloned().expect("replacement order for the selected occurrence");
    assert!(matches!(choice.kind, RequestKind::ChooseReplacement { life: Some(-1), .. }));
    g.answer(choice.id, FIRST, Answer::Index(0)).unwrap(); g.run();
    let request = g.pending().expect("choose between the modified and the other occurrence");
    let RequestKind::OrderReplacementOccurrences { occurrences } = &request.kind else { panic!("occurrence order: {:?}", request.kind) };
    assert!(occurrences.iter().any(|o| matches!(o, ReplacementOccurrence::Life { amount: -2, occurrence, .. } if *occurrence == ids[1])));
    assert!(occurrences.iter().any(|o| matches!(o, ReplacementOccurrence::Life { amount: -1, occurrence, .. } if *occurrence == ids[0])));
}

fn ask_controller_after_loss(g: &mut dyn cantrip_api::Game, asker: &Asker, _: &Proposal, _: &Proposal) {
    let _ = g.ask(asker.controller, RequestKind::May { source: asker.source, purpose: Purpose::Ask(&OPTIONAL_EXILE) });
}
static ASK_CONTROLLER_ON_LOSS: CardDef = CardDef {
    name: "Life replacement asks its controller", types: TypeSet::of(&[CardType::Enchantment]),
    replacements: &[replacement(ReplacementKind::LifeChange, opponent_loses_on_your_turn, twice,
        "An opponent loses twice that life; the controller chooses.").then(ask_controller_after_loss)],
    ..CardDef::BLANK
};

#[test]
fn mixed_forced_replacement_completes_contained_choice_before_outer_choice() {
    let mut g = game();
    put(&mut g, &ASK_CONTROLLER_ON_LOSS, ACTIVE);
    put(&mut g, &LAND_EXILER, ACTIVE); put(&mut g, &LAND_EXILER, ACTIVE);
    put(&mut g, &LAND_PAYMENT_LAND, FIRST);
    let card = g.cards.id_of(&SAME_PAYMENT);
    let source = g.state.push_obj(Obj::new(card, ACTIVE, Zone::STACK));
    g.state.zone_push(Zone::STACK, source);
    g.state.stack_push(crate::actions::stack_item(StackKind::Spell, source, AbilityRef::spell(card), ACTIVE, &[]));
    let serial = g.state.stack.last().unwrap().serial;
    g.state.frame_push(Frame::new(FrameKind::Resolve { serial }));
    g.run();
    let request = g.pending().cloned().expect("contained continuation choice");
    assert_eq!(request.player, ACTIVE, "the forced outer replacement runs its contained instruction");
    assert!(matches!(request.kind, RequestKind::May { .. }));
    g.answer(request.id, ACTIVE, Answer::Yes).unwrap(); g.run();
    let request = g.pending().expect("outer land replacement choice");
    assert_eq!(request.player, FIRST);
    assert!(matches!(request.kind, RequestKind::ChooseReplacement { subject: Some(_), .. }));
}

fn no_choice_after_land_replacement(_: &mut dyn cantrip_api::Game, _: &Asker, _: &Proposal, _: &Proposal) {}
static MANDATORY_LAND_WITH_THEN: CardDef = CardDef {
    name: "Mandatory land replacement with a forced continuation", types: TypeSet::of(&[CardType::Enchantment]),
    replacements: &[replacement(ReplacementKind::ZoneChange, land_to_graveyard, exile_land_instead,
        "Exile it instead and do a mandatory action.").then(no_choice_after_land_replacement)],
    ..CardDef::BLANK
};

#[test]
fn mixed_forced_continuations_do_not_ask_a_fake_order_question() {
    let mut g = game();
    put(&mut g, &MANDATORY_LAND_WITH_THEN, ACTIVE);
    put(&mut g, &LAND_PAYMENT_LAND, FIRST); put(&mut g, &LAND_PAYMENT_LAND, FIRST);
    let card = g.cards.id_of(&MIXED_PAYMENT);
    let source = g.state.push_obj(Obj::new(card, ACTIVE, Zone::STACK));
    g.state.zone_push(Zone::STACK, source);
    g.state.stack_push(crate::actions::stack_item(StackKind::Spell, source, AbilityRef::spell(card), ACTIVE, &[]));
    let serial = g.state.stack.last().unwrap().serial;
    g.state.frame_push(Frame::new(FrameKind::Resolve { serial }));
    g.run();
    assert!(!g.pending().is_some_and(|request| matches!(request.kind, RequestKind::OrderReplacementOccurrences { .. })),
        "mandatory continuations make no choice to order");
    assert_eq!(g.state.zone(Zone::exile(FIRST)).len(), 2);
}

#[test]
fn mixed_single_optional_replacements_are_ordered_by_their_controller() {
    let mut g = game();
    put(&mut g, &OPTIONAL_LAND_EXILER, ACTIVE);
    put(&mut g, &LAND_PAYMENT_LAND, FIRST); put(&mut g, &LAND_PAYMENT_LAND, FIRST);
    let card = g.cards.id_of(&SAME_PAYMENT);
    let source = g.state.push_obj(Obj::new(card, ACTIVE, Zone::STACK));
    g.state.zone_push(Zone::STACK, source);
    g.state.stack_push(crate::actions::stack_item(StackKind::Spell, source, AbilityRef::spell(card), ACTIVE, &[]));
    let serial = g.state.stack.last().unwrap().serial;
    g.state.frame_push(Frame::new(FrameKind::Resolve { serial }));
    g.run();
    let before = g.at;
    let request = g.pending().cloned().expect("order optional replacement decisions");
    assert_eq!(request.player, ACTIVE, "the controller makes the optional decisions, not the affected land controller");
    assert!(matches!(request.kind, RequestKind::OrderReplacementOccurrences { .. }));
    g.answer(request.id, ACTIVE, Answer::CostPayment(0)).unwrap(); g.run();
    let request = g.pending().cloned().expect("first optional decision");
    assert_eq!(request.player, ACTIVE);
    assert!(matches!(request.kind, RequestKind::May { .. }));
    g.answer(request.id, ACTIVE, Answer::No).unwrap(); g.run();
    let request = g.pending().cloned().expect("second optional decision");
    assert_eq!(request.player, ACTIVE);
    assert!(matches!(request.kind, RequestKind::May { .. }));
    g.answer(request.id, ACTIVE, Answer::Yes).unwrap(); g.run();
    let after = g.at;
    assert_eq!(g.state.zone(Zone::graveyard(FIRST)).len(), 1);
    assert_eq!(g.state.zone(Zone::exile(FIRST)).len(), 1);
    g.goto(before);
    assert!(matches!(g.pending().map(|r| &r.kind), Some(RequestKind::OrderReplacementOccurrences { .. })));
    g.goto(after);
    assert_eq!(g.state.zone(Zone::graveyard(FIRST)).len(), 1);
    assert_eq!(g.state.zone(Zone::exile(FIRST)).len(), 1);
}
