//! A permission to cast a spell as though it had flash, and a ban on casting spells with some qualities, judged on the spell
//! the card would be on the stack under its caster (CR 601.2a, 601.3, 601.3b, 601.3e), over test cards: a static ability that
//! works on spells alone ("creature spells you control are Spirits", the spell clause of Arcane Adaptation, Conspiracy and
//! Maskwood Nexus) makes a creature card in a hand a Spirit spell once it would be cast, though not where it is.

use crate::actions::{cast_ways_during_resolution, priority_options, stack_item};
use crate::{Cx, Game, Setup};
use cantrip_api::*;

const YOU: PlayerId = PlayerId(0);

fn nothing(_: &mut dyn cantrip_api::Game, _: &StackItem) -> Result<(), Suspend> {
    Ok(())
}

/// "Creature spells you control are Spirits": a creature spell on the stack its source's controller controls.
fn your_creature_spells(q: &dyn LayerQuery, source: ObjectId, affected: ObjectId, so_far: &Chars) -> bool {
    let (s, o) = (q.state(), q.state().obj(affected));
    so_far.is(CardType::Creature) && o.zone.is(ZoneKind::Stack) && o.controller == s.obj(source).controller
}

fn spirit(_: &dyn LayerQuery, _: ObjectId, c: &mut Chars) {
    c.subtypes = c.subtypes.with(Subtype::Spirit);
}

static BANNER: CardDef = CardDef {
    name: "Cast As Spell Test Banner",
    types: TypeSet::of(&[CardType::Enchantment]),
    statics: &[static_ability(Layer::Type, your_creature_spells, spirit, "Creature spells you control are Spirits.").everywhere()],
    ..CardDef::BLANK
};

/// "You may cast Spirit spells as though they had flash" (Rattlechains's).
fn spirit_you_cast(q: &dyn Query, asker: &Asker, spell: ObjectId, caster: PlayerId) -> bool {
    caster == asker.controller && q.chars(spell).has_subtype(Subtype::Spirit)
}

static FLASHER: CardDef =
    CardDef { name: "Cast As Spell Test Flasher", types: TypeSet::of(&[CardType::Enchantment]), flash_rules: &[spirit_you_cast], ..CardDef::BLANK };

/// "Spirit spells can't be cast."
fn no_spirits(q: &dyn Query, _: &Asker, _: PlayerId, spell: ObjectId) -> bool {
    !q.chars(spell).has_subtype(Subtype::Spirit)
}

static WARDEN: CardDef =
    CardDef { name: "Cast As Spell Test Warden", types: TypeSet::of(&[CardType::Enchantment]), spell_rules: &[no_spirits], ..CardDef::BLANK };

static BEAR: CardDef = CardDef {
    name: "Cast As Spell Test Bear",
    types: TypeSet::of(&[CardType::Creature]),
    power: Some(2),
    toughness: Some(2),
    spell: spell(&[], nothing),
    ..CardDef::BLANK
};

/// A bear that costs {1}, cast with no mana: only for nothing, or for {1} less.
static CUB: CardDef = CardDef {
    name: "Cast As Spell Test Cub",
    types: TypeSet::of(&[CardType::Creature]),
    cost: mana("{1}"),
    power: Some(1),
    toughness: Some(1),
    spell: spell(&[], nothing),
    ..CardDef::BLANK
};

/// "You may cast Spirit spells without paying their mana costs" (Dracogenesis's, for Dragons).
static FREEDOM: CardDef = CardDef {
    name: "Cast As Spell Test Freedom",
    types: TypeSet::of(&[CardType::Enchantment]),
    spell_alternatives: &[SpellAlternative { text: "Without paying its mana cost", applies: spirit_you_cast_free, cost: InsteadOfManaCost::WithoutPaying, flash: false, kind: WayKind::Other }],
    ..CardDef::BLANK
};

fn spirit_you_cast_free(q: &dyn Query, asker: &Asker, spell: ObjectId, player: PlayerId) -> bool {
    player == asker.controller && q.chars(spell).has_subtype(Subtype::Spirit)
}

/// "Spirit spells you cast cost {1} less to cast" (Herald's Horn's, for the chosen type).
fn spirits_cost_less(q: &dyn Query, source: ObjectId, player: PlayerId, costed: Costed, _: &[Target]) -> CostDelta {
    match costed {
        Costed::Spell(spell) if player == q.controller(source) && q.chars(spell).has_subtype(Subtype::Spirit) => {
            CostDelta { less: mana("{1}"), ..CostDelta::NONE }
        }
        _ => CostDelta::NONE,
    }
}

static HORN: CardDef =
    CardDef { name: "Cast As Spell Test Horn", types: TypeSet::of(&[CardType::Artifact]), cost_changes: &[spirits_cost_less], ..CardDef::BLANK };

static CASTER: CardDef = CardDef { name: "Cast As Spell Test Caster", types: TypeSet::of(&[CardType::Sorcery]), ..CardDef::BLANK };

/// A game with the bear in your hand and `permanents` of yours on the battlefield, in a main phase of `active`'s with you
/// holding priority.
fn game(permanents: &[&'static CardDef], active: PlayerId) -> (Game, ObjectId) {
    game_with(&BEAR, permanents, active)
}

/// `game` with `hand` in your hand for the bear.
fn game_with(hand: &'static CardDef, permanents: &[&'static CardDef], active: PlayerId) -> (Game, ObjectId) {
    let table: &'static CardTable = Box::leak(Box::new(CardTable::new([&BANNER, &FLASHER, &WARDEN, &BEAR, &CUB, &FREEDOM, &HORN, &CASTER])));
    let mut game = Game::new(table, &Setup { decks: vec![Vec::new(), Vec::new()], ..Default::default() });
    let put = |game: &mut Game, def: &'static CardDef, zone: Zone| {
        let id = game.state.push_obj(Obj::new(table.id_of(def), YOU, zone));
        game.state.zone_push(zone, id);
        id
    };
    let bear = put(&mut game, hand, Zone::hand(YOU));
    for &def in permanents {
        put(&mut game, def, Zone::BATTLEFIELD);
    }
    game.state.see_statics();
    game.state.update_turn(|t| {
        t.active = active;
        t.priority = YOU;
        t.set_step(Step::PrecombatMain);
    });
    (game, bear)
}

fn cx(game: &mut Game) -> Cx<'_> {
    Cx { g: game, frame: usize::MAX, cursor: 0, mana_source: None, entering: None, lineage: Vec::new(), damage: None, asked_at: None, doer: None, producing: None }
}

/// Whether you are offered to cast the bear with priority.
fn offered(permanents: &[&'static CardDef], active: PlayerId) -> bool {
    let (mut game, bear) = game(permanents, active);
    priority_options(&mut cx(&mut game), YOU).iter().any(|o| o.kind == ActionKind::CastSpell && o.source == Some(bear))
}

/// Whether an effect of yours resolving could cast the bear (CR 608.2g).
fn castable_during_resolution(permanents: &[&'static CardDef]) -> bool {
    let (mut game, bear) = game(permanents, YOU);
    let caster = game.cards.find("Cast As Spell Test Caster").unwrap();
    let id = game.state.push_obj(Obj::new(caster, YOU, Zone::STACK));
    game.state.zone_push(Zone::STACK, id);
    game.state.stack_push(stack_item(StackKind::Spell, id, AbilityRef { card: caster, index: 0 }, YOU, &[]));
    let serial = game.state.stack.last().expect("the caster").serial;
    game.state.frame_push(cantrip_state::Frame::new(cantrip_state::FrameKind::Resolve { serial }));
    let frame = game.state.agenda.len() - 1;
    let mut resolving = Cx { g: &mut game, frame, cursor: 0, mana_source: None, entering: None, lineage: Vec::new(), damage: None, asked_at: None, doer: None, producing: None };
    !cast_ways_during_resolution(&mut resolving, bear, YOU).is_empty()
}

/// CR 601.3b, 601.3e, 702.8: "You may cast Spirit spells as though they had flash" lets the bear be cast on the opponent's
/// turn when, as your spell, it would be a Spirit; in your hand it is none.
#[test]
fn a_flash_permission_reads_the_spell_the_card_would_be() {
    assert!(!offered(&[&FLASHER], PlayerId(1)), "no Spirit spell, no flash");
    assert!(offered(&[&FLASHER, &BANNER], PlayerId(1)), "a Spirit as your spell, so cast as though it had flash");
    assert!(!offered(&[&BANNER], PlayerId(1)), "a Spirit spell with no permission is cast at sorcery speed");
}

/// CR 101.2, 601.3, 601.3e: "Spirit spells can't be cast" forbids the bear, a Spirit as your spell, though it is none in your
/// hand - with priority and as an effect would cast it.
#[test]
fn a_ban_on_spells_reads_the_spell_the_card_would_be() {
    assert!(offered(&[&WARDEN], YOU), "no Spirit spell, nothing banned");
    assert!(!offered(&[&WARDEN, &BANNER], YOU), "a Spirit as your spell, so banned");
    assert!(castable_during_resolution(&[&WARDEN]));
    assert!(!castable_during_resolution(&[&WARDEN, &BANNER]));
}

/// Whether you are offered to cast the cub, which costs {1}, with no mana, in your main phase.
fn cub_offered(permanents: &[&'static CardDef]) -> bool {
    let (mut game, cub) = game_with(&CUB, permanents, YOU);
    priority_options(&mut cx(&mut game), YOU).iter().any(|o| o.kind == ActionKind::CastSpell && o.source == Some(cub))
}

/// CR 118.9, 601.2b, 601.3e: "You may cast Spirit spells without paying their mana costs" lets the cub, a Spirit as your
/// spell, be cast for nothing, though it is none in your hand.
#[test]
fn an_alternative_cost_reads_the_spell_the_card_would_be() {
    assert!(!cub_offered(&[&FREEDOM]), "no Spirit spell, no alternative cost, and no mana for {{1}}");
    assert!(cub_offered(&[&FREEDOM, &BANNER]), "a Spirit as your spell, cast without paying its mana cost");
}

/// CR 601.2f, 601.3e: "Spirit spells you cast cost {1} less" takes the cub's {1} off where it would be a Spirit as your
/// spell, though it is none in your hand: a cast that is affordable only so is offered.
#[test]
fn a_cost_change_reads_the_spell_the_card_would_be() {
    assert!(!cub_offered(&[&HORN]), "no Spirit spell, no reduction, and no mana for {{1}}");
    assert!(cub_offered(&[&HORN, &BANNER]), "a Spirit as your spell, so it costs nothing");
}
