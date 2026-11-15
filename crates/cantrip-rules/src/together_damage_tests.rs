//! Damage as a part of one instruction several players carry out at once (`Game::together`, CR 101.4, 120.2), over test
//! cards shaped like Worms of the Earth's "any player may sacrifice two lands of their choice or have this enchantment deal 5
//! damage to that player": the damage and the sacrifices are one event. The damage is formed - prevented, replaced - and its
//! results prepared against the game as it is before any part happens (CR 120.4c, 616.1), so a land sacrificed in it still
//! prevents it, or doubles the life it makes a player lose; it is dealt by its own
//! procedure (lifelink, CR 702.15b); and its triggers are looked for once every part has happened, so a land with a damage
//! trigger sacrificed in the same event doesn't trigger (CR 603.10), where one that stays does.

use crate::{Cx, Game, Setup};
use cantrip_api::Game as Rules;
use cantrip_api::*;
use cantrip_state::{Frame, FrameKind};

const P0: PlayerId = PlayerId(0);
const P1: PlayerId = PlayerId(1);

fn nothing(_: &mut dyn cantrip_api::Game, _: &StackItem) -> Result<(), Suspend> {
    Ok(())
}

/// "Whenever one or more opponents are dealt noncombat damage" (Master of Barbs's trigger).
fn opponent_dealt_damage(q: &dyn Query, me: &Asker, event: &Event) -> Option<Fired> {
    match event {
        Event::Damage { target: Target::Player(p), amount, .. } if *amount > 0 && q.state().opponents(me.controller).any(|o| o == *p) => {
            Fired::PLAIN
        }
        _ => None,
    }
}

const BARBS: &[AbilityDef] = &[triggered(
    when_asked(EventMask::DAMAGE, opponent_dealt_damage),
    nothing,
    "Whenever one or more opponents are dealt noncombat damage, creatures you control get +1/+0 until end of turn.",
)
.once_per_batch()];

/// A land with the trigger: a Master of Barbs made a land (Ashaya, Soul of the Wild).
static BARBED_LAND: CardDef =
    CardDef { name: "Together Damage Barbed Land", has_cost: false, types: TypeSet::of(&[CardType::Land]), abilities: BARBS, ..CardDef::BLANK };
/// The same trigger on a permanent that stays.
static BARBED: CardDef =
    CardDef { name: "Together Damage Barbed", types: TypeSet::of(&[CardType::Enchantment]), abilities: BARBS, ..CardDef::BLANK };

static LAND: CardDef = CardDef { name: "Together Damage Land", has_cost: false, types: TypeSet::of(&[CardType::Land]), ..CardDef::BLANK };

static SOURCE: CardDef = CardDef { name: "Together Damage Source", types: TypeSet::of(&[CardType::Enchantment]), ..CardDef::BLANK };
static LIFELINKER: CardDef = CardDef {
    name: "Together Damage Lifelinker",
    types: TypeSet::of(&[CardType::Enchantment]),
    keywords: Keywords::of(&[Keyword::Lifelink]),
    ..CardDef::BLANK
};

fn damage_to_a_player(_: &dyn Query, _: &Asker, p: &Proposal) -> bool {
    matches!(p, Proposal::Damage(d) if matches!(d.target, Target::Player(_)) && d.amount > 0)
}

fn prevent_it(_: &dyn Query, _: &Asker, p: &mut Proposal) {
    if let Proposal::Damage(d) = p {
        d.amount = 0;
    }
}

/// A land with "prevent all damage that would be dealt to players".
static SHELTER_LAND: CardDef = CardDef {
    name: "Together Damage Shelter Land",
    has_cost: false,
    types: TypeSet::of(&[CardType::Land]),
    replacements: &[replacement(ReplacementKind::Damage, damage_to_a_player, prevent_it, "Prevent all damage that would be dealt to players.")],
    ..CardDef::BLANK
};

fn opponent_loses_on_your_turn(q: &dyn Query, me: &Asker, proposal: &Proposal) -> bool {
    matches!(proposal, Proposal::LifeChange(l) if l.amount < 0 && l.player != me.controller && q.state().turn.active == me.controller)
}

fn twice(_: &dyn Query, _: &Asker, proposal: &mut Proposal) {
    if let Proposal::LifeChange(l) = proposal {
        l.amount *= 2;
    }
}

/// A land with Bloodletter of Aclazotz's "If an opponent would lose life during your turn, they lose twice that much life
/// instead" (a Bloodletter made a Forest by Ashaya, Soul of the Wild).
static DOUBLER_LAND: CardDef = CardDef {
    name: "Together Damage Doubler Land",
    has_cost: false,
    types: TypeSet::of(&[CardType::Land]),
    replacements: &[replacement(ReplacementKind::LifeChange, opponent_loses_on_your_turn, twice,
        "If an opponent would lose life during your turn, they lose twice that much life instead.")],
    ..CardDef::BLANK
};

fn game() -> Game {
    let table = Box::leak(Box::new(CardTable::new([&BARBED_LAND, &BARBED, &LAND, &SOURCE, &LIFELINKER, &SHELTER_LAND, &DOUBLER_LAND])));
    let mut g = Game::new(table, &Setup { decks: vec![Vec::new(); 2], ..Default::default() });
    while !g.state.agenda.is_empty() {
        g.state.frame_remove(0);
    }
    g.state.update_turn(|t| {
        t.active = P0;
        t.priority = P0;
        t.set_step(Step::Upkeep);
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
    Cx { g, frame: 0, cursor: 0, mana_source: None, entering: None, lineage: Vec::new(), damage: None, asked_at: None, doer: None, producing: None, reached: None }
}

fn triggers_of(g: &Game, source: ObjectId) -> usize {
    g.state.triggers.iter().filter(|t| t.source == source).count()
        + g.state.stack.iter().filter(|i| i.kind == StackKind::Triggered && i.object == source).count()
}

/// P0 sacrifices the barbed land and a land as P1 is dealt 5 damage, one event: P1 is dealt it, the barbed land is in the
/// graveyard and never triggered (CR 603.10), and the barbed enchantment that stays triggered once.
#[test]
fn a_land_sacrificed_with_the_damage_does_not_see_it() {
    let mut g = game();
    let barbed_land = put(&mut g, &BARBED_LAND, P0);
    let land = put(&mut g, &LAND, P0);
    let barbed = put(&mut g, &BARBED, P0);
    let source = put(&mut g, &SOURCE, P0);
    let mut c = cx(&mut g);
    c.together(Together { sacrifices: &[(P0, barbed_land), (P0, land)], damage: &[(source, Target::Player(P1), 5)], ..Together::default() });
    assert_eq!(g.state.player(P1).life, 15);
    assert!(g.state.obj(barbed_land).became.is_some_and(|new| g.state.obj(new).zone.is(ZoneKind::Graveyard)));
    assert_eq!(triggers_of(&g, barbed_land), 0, "it left in the same event as the damage");
    assert_eq!(triggers_of(&g, barbed), 1);
}

/// The damage alone, nothing sacrificed: the barbed land triggers.
#[test]
fn a_land_that_stays_sees_the_damage() {
    let mut g = game();
    let barbed_land = put(&mut g, &BARBED_LAND, P0);
    let source = put(&mut g, &SOURCE, P0);
    let mut c = cx(&mut g);
    c.together(Together { damage: &[(source, Target::Player(P1), 5)], ..Together::default() });
    assert_eq!(g.state.player(P1).life, 15);
    assert_eq!(triggers_of(&g, barbed_land), 1);
}

/// The damage is formed against the game before any part happens (CR 101.4, 616.1): P1's shelter land, sacrificed in the
/// same event, still prevents the damage to P0. And it is dealt by its own procedure: a lifelink source's controller gains
/// what it dealt (CR 702.15b).
#[test]
fn the_damage_is_judged_before_the_sacrifices_and_dealt_as_damage_is() {
    let mut g = game();
    let shelter = put(&mut g, &SHELTER_LAND, P1);
    let land = put(&mut g, &LAND, P1);
    let source = put(&mut g, &SOURCE, P1);
    let mut c = cx(&mut g);
    c.together(Together { sacrifices: &[(P1, shelter), (P1, land)], damage: &[(source, Target::Player(P0), 5)], ..Together::default() });
    assert_eq!(g.state.player(P0).life, 20);
    assert!(g.state.obj(shelter).became.is_some());

    let mut g = game();
    let land = put(&mut g, &LAND, P1);
    let lifelinker = put(&mut g, &LIFELINKER, P1);
    let mut c = cx(&mut g);
    c.together(Together { sacrifices: &[(P1, land)], damage: &[(lifelinker, Target::Player(P0), 5)], ..Together::default() });
    assert_eq!(g.state.player(P0).life, 15);
    assert_eq!(g.state.player(P1).life, 25);
}

/// The damage's results are prepared against the game before any part happens too (CR 101.4, 120.4c, 616.1): P0's doubler
/// land, sacrificed in the same event on P0's turn, still doubles the life P1 loses to the damage - 10, not 5.
#[test]
fn the_damage_results_are_judged_before_the_sacrifices() {
    let mut g = game();
    let doubler = put(&mut g, &DOUBLER_LAND, P0);
    let source = put(&mut g, &SOURCE, P0);
    let mut c = cx(&mut g);
    c.together(Together { sacrifices: &[(P0, doubler)], damage: &[(source, Target::Player(P1), 5)], ..Together::default() });
    assert!(g.state.obj(doubler).became.is_some());
    assert_eq!(g.state.player(P1).life, 10);
}
