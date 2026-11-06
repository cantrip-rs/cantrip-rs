//! Rules that change the quantity assigned in combat, without changing power (CR 510.1a, 609.4), and the ability of a
//! creature to assign by toughness, printed or gained (`Keyword::AssignsDamageByToughness`, CR 613.1f).

use cantrip_api::*;
use crate::{Cx, Game, static_rules};

const ME: PlayerId = PlayerId(0);
const THEM: PlayerId = PlayerId(1);

fn itself(_: &dyn Query, asker: &Asker, creature: ObjectId) -> bool {
    asker.source == creature
}

fn every_creature(_: &dyn Query, _: &Asker, _: ObjectId) -> bool {
    true
}

static NEGATIVE: CardDef = CardDef {
    name: "Assignment Test Negative",
    types: TypeSet::of(&[CardType::Creature]),
    power: Some(-2),
    toughness: Some(4),
    ..CardDef::BLANK
};
static ABSOLUTE: CardDef = CardDef {
    name: "Assignment Test Absolute",
    damage_by_absolute_power: &[itself],
    ..NEGATIVE
};
static TOUGHNESS: CardDef = CardDef {
    name: "Assignment Test Toughness",
    types: TypeSet::of(&[CardType::Enchantment]),
    damage_by_toughness: &[every_creature],
    ..CardDef::BLANK
};

fn cx(game: &mut Game) -> Cx<'_> {
    Cx { g: game, frame: usize::MAX, cursor: 0, mana_source: None, entering: None, lineage: Vec::new(), damage: None, asked_at: None, doer: None, producing: None }
}

fn table(cards: &[(&'static CardDef, PlayerId)]) -> (Game, Vec<ObjectId>) {
    let table: &'static CardTable = Box::leak(Box::new(CardTable::new([&NEGATIVE, &ABSOLUTE, &TOUGHNESS])));
    let mut game = Game::new(table, &crate::Setup { decks: vec![Vec::new(), Vec::new()], ..Default::default() });
    let ids = cards.iter().map(|&(def, owner)| {
        let id = game.state.push_obj(Obj::new(table.id_of(def), owner, Zone::BATTLEFIELD));
        game.state.zone_push(Zone::BATTLEFIELD, id);
        id
    }).collect();
    (game, ids)
}

/// CR 510.1a, 609.4, 107.1b: assignment alone reads negative power as positive; zero and positive power are unchanged.
/// Even the minimum signed power has an exact unsigned magnitude, and an ordinary creature keeps assigning zero.
#[test]
fn absolute_power_assignment_preserves_actual_power_across_the_signed_range() {
    for power in [i32::MIN, -6, -2, 0, 3, i32::MAX] {
        let (mut game, ids) = table(&[(&ABSOLUTE, ME), (&NEGATIVE, ME)]);
        let card = game.cards.id_of(&ABSOLUTE);
        game.state.update_obj(ids[0], |o| o.copy = Some(Copiable::of(card).sized(power, 4)));
        let cx = cx(&mut game);
        assert_eq!(cx.chars(ids[0]).power, Some(power));
        assert_eq!(static_rules::combat_damage_of(&cx, ids[0]), power.unsigned_abs());
        assert_eq!(static_rules::combat_damage_of(&cx, ids[1]), 0);
    }
}

/// CR 707.2, 613.1f, 702.26b: copied static abilities apply, lost abilities do not, and phased-out sources are absent.
#[test]
fn absolute_power_assignment_follows_copies_ability_loss_and_phasing() {
    let (mut game, ids) = table(&[(&NEGATIVE, ME)]);
    let creature = ids[0];
    let card = game.cards.id_of(&ABSOLUTE);
    let mut cx = cx(&mut game);
    cantrip_api::Game::set_copy(&mut cx, creature, Copiable::of(card));
    assert_eq!(static_rules::combat_damage_of(&cx, creature), 2);
    cantrip_api::Game::add_effect(&mut cx, creature, creature, EffectKind::LoseAllAbilities, Duration::EndOfTurn);
    assert_eq!(static_rules::combat_damage_of(&cx, creature), 0);
    cx.st().effects_retain(|_| false);
    crate::phasing::phase_out(&mut cx, &[creature], None, false);
    assert_eq!(static_rules::combat_damage_of(&cx, creature), 0);
    crate::phasing::untap_step(&mut cx, ME);
    assert_eq!(static_rules::combat_damage_of(&cx, creature), 2);
    assert_eq!(cx.chars(creature).power, Some(-2));
}

/// CR 609.4: the power exception has no effect when another rule assigns damage using toughness. Negative toughness
/// still assigns zero rather than being made positive by a rule about power.
#[test]
fn assignment_by_toughness_ignores_the_exception_about_power() {
    let (mut game, ids) = table(&[(&ABSOLUTE, ME), (&TOUGHNESS, ME)]);
    let card = game.cards.id_of(&ABSOLUTE);
    assert_eq!(static_rules::combat_damage_of(&cx(&mut game), ids[0]), 4);
    game.state.update_obj(ids[0], |o| o.copy = Some(Copiable::of(card).sized(-2, -3)));
    assert_eq!(static_rules::combat_damage_of(&cx(&mut game), ids[0]), 0);
    cantrip_api::Game::add_effect(&mut cx(&mut game), ids[1], ids[1], EffectKind::LoseAllAbilities, Duration::EndOfTurn);
    assert_eq!(static_rules::combat_damage_of(&cx(&mut game), ids[0]), 2);
}

/// CR 510.1a-d: the same assignment rule applies unblocked, blocked and blocking, while an ordinary creature with
/// negative power deals no combat damage. No assignment changes the power that a later effect reads.
#[test]
fn absolute_power_is_used_for_unblocked_attackers_blocked_attackers_and_blockers() {
    for arrangement in 0..3 {
        let blocking = arrangement == 2;
        let (mut game, ids) = table(&[(&ABSOLUTE, if blocking { THEM } else { ME }), (&NEGATIVE, if blocking { ME } else { THEM })]);
        let [absolute, normal] = ids[..] else { unreachable!() };
        let attacker = if blocking { normal } else { absolute };
        let blocker = if blocking { absolute } else { normal };
        game.state.set_attackers(vec![Attacker { blocked: arrangement > 0, ..Attacker::new(attacker, Target::Player(THEM), THEM, TypeSet::NONE) }]);
        if arrangement > 0 {
            game.state.set_blocks(vec![Block { blocker, attacker, first_strike: false }]);
        }
        game.state.update_turn(|t| t.set_step(Step::CombatDamage));
        crate::turn::begin_step(&mut cx(&mut game)).expect("no assignment choice");
        assert_eq!(game.state.player(THEM).life, if arrangement == 0 { 18 } else { 20 });
        assert_eq!(game.state.obj(normal).damage, if arrangement == 0 { 0 } else { 2 });
        assert_eq!(game.state.obj(absolute).damage, 0);
        assert_eq!(cx(&mut game).chars(absolute).power, Some(-2));
    }
}

/// CR 510.1a, 704.5g: the minimum signed power assigns 2,147,483,648 damage, which is lethal to a positive-toughness blocker.
/// Marked damage must not become negative when the next state-based check compares it with toughness.
#[test]
fn minimum_signed_power_deals_its_exact_magnitude_and_destroys_its_blocker() {
    let (mut game, ids) = table(&[(&ABSOLUTE, ME), (&NEGATIVE, THEM)]);
    let [attacker, blocker] = ids[..] else { unreachable!() };
    let card = game.cards.id_of(&ABSOLUTE);
    game.state.update_obj(attacker, |o| o.copy = Some(Copiable::of(card).sized(i32::MIN, 4)));
    game.state.set_attackers(vec![Attacker { blocked: true, ..Attacker::new(attacker, Target::Player(THEM), THEM, TypeSet::NONE) }]);
    game.state.set_blocks(vec![Block { blocker, attacker, first_strike: false }]);
    game.state.update_turn(|t| t.set_step(Step::CombatDamage));
    crate::turn::begin_step(&mut cx(&mut game)).expect("no assignment choice");
    assert_eq!(game.state.obj(blocker).damage, 2_147_483_648);
    assert_eq!(cx(&mut game).chars(attacker).power, Some(i32::MIN));
    crate::settle::state_based_actions_alone(&mut cx(&mut game)).expect("no choices");
    assert!(!crate::live(&game.state, blocker));
    assert!(crate::live(&game.state, attacker));
    let graveyard = game.state.zone(Zone::graveyard(THEM));
    assert_eq!(graveyard.len(), 1);
    assert_eq!(game.state.obj(graveyard[0]).card, game.cards.id_of(&NEGATIVE));
}

/// CR 702.19b: an indestructible blocker already marked with at least 32,768 damage needs zero more assigned as lethal.
/// The assignment prompt and actual trample damage use that exact subtraction up to u32::MAX, even with deathtouch;
/// with no marked damage, deathtouch needs one instead.
#[test]
fn trample_accounts_for_large_marked_damage_and_deathtouch_without_signed_narrowing() {
    for (marked, deathtouch, to_blocker) in [(32768, false, 0), (u32::MAX, false, 0), (32768, true, 0), (u32::MAX, true, 0), (0, true, 1)] {
        let (mut game, ids) = table(&[(&ABSOLUTE, ME), (&NEGATIVE, THEM)]);
        let [attacker, blocker] = ids[..] else { unreachable!() };
        cantrip_api::Game::add_effect(&mut cx(&mut game), attacker, attacker, EffectKind::AddKeywords(Keywords::of(&[Keyword::Trample])), Duration::EndOfTurn);
        if deathtouch {
            cantrip_api::Game::add_effect(&mut cx(&mut game), attacker, attacker, EffectKind::AddKeywords(Keywords::of(&[Keyword::Deathtouch])), Duration::EndOfTurn);
        }
        cantrip_api::Game::add_effect(&mut cx(&mut game), blocker, blocker, EffectKind::AddKeywords(Keywords::of(&[Keyword::Indestructible])), Duration::EndOfTurn);
        game.state.update_obj(blocker, |o| o.damage = marked);
        game.state.set_attackers(vec![Attacker { blocked: true, ..Attacker::new(attacker, Target::Player(THEM), THEM, TypeSet::NONE) }]);
        game.state.set_blocks(vec![Block { blocker, attacker, first_strike: false }]);
        game.state.update_turn(|t| t.set_step(Step::CombatDamage));
        let frame = game.state.agenda.len();
        game.state.frame_push(cantrip_state::Frame::new(cantrip_state::FrameKind::BeginStep));
        let mut cx = cx(&mut game);
        cx.frame = frame;
        assert!(matches!(crate::turn::begin_step(&mut cx), Err(crate::Stop::Suspend)));
        let request = cx.s().agenda[frame].pending.clone().expect("a combat assignment");
        let RequestKind::AssignCombatDamage { divisions } = &request.kind else { panic!("combat assignment expected") };
        assert_eq!(divisions.len(), 1);
        assert_eq!(divisions[0].lethal, [to_blocker]);
        assert_eq!(divisions[0].suggested, [to_blocker]);
        assert_eq!(divisions[0].suggested_to_defender, 2 - to_blocker);
        let answer = Answer::Shares(DamageDivision::suggested_shares(divisions));
        crate::check_answer(cx.g, &request, &answer).expect("all damage past an already lethally marked blocker is legal");
        cx.st().update_frame(frame, |f| {
            f.pending = None;
            f.answers.push((None, answer));
        });
        crate::turn::begin_step(&mut cx).expect("the assignment was answered");
        assert_eq!(cx.s().player(THEM).life, 18 + to_blocker as i32);
        assert_eq!(cx.s().obj(blocker).damage, marked + to_blocker);
        crate::settle::state_based_actions_alone(&mut cx).expect("no choices");
        assert!(crate::live(cx.s(), blocker));
    }
}

/// "This creature assigns combat damage equal to its toughness rather than its power", to give.
const BY_TOUGHNESS: Keywords = Keywords::of(&[Keyword::AssignsDamageByToughness]);

fn gains() -> EffectKind {
    EffectKind::AddKeywords(BY_TOUGHNESS)
}

fn creatures(_: &dyn LayerQuery, _: ObjectId, _: ObjectId, chars: &Chars) -> bool {
    chars.is(CardType::Creature)
}

fn everything(_: &dyn LayerQuery, _: ObjectId, _: ObjectId, _: &Chars) -> bool {
    true
}

fn give_by_toughness(_: &dyn LayerQuery, _: ObjectId, chars: &mut Chars) {
    chars.keywords = Keywords(chars.keywords.0 | BY_TOUGHNESS.0);
}

fn set_plains(_: &dyn LayerQuery, _: ObjectId, chars: &mut Chars) {
    chars.set_land_subtypes(SubtypeSet::of(&[Subtype::Plains]));
}

/// A creature with no such ability of its own.
static PLAIN: CardDef = CardDef {
    name: "Assignment Test Plain",
    types: TypeSet::of(&[CardType::Creature]),
    power: Some(1),
    toughness: Some(3),
    ..CardDef::BLANK
};
/// One that prints the ability.
static PRINTED: CardDef = CardDef { name: "Assignment Test Printed", keywords: BY_TOUGHNESS, ..PLAIN };
/// A land creature that prints it, whose rules text a land type can take (CR 305.7).
static PRINTED_LAND: CardDef = CardDef { name: "Assignment Test Printed Land", types: TypeSet::of(&[CardType::Land, CardType::Creature]), ..PRINTED };
static STRIPPER: CardDef = CardDef {
    name: "Assignment Test Stripper",
    types: TypeSet::of(&[CardType::Enchantment]),
    statics: &[loses_all_abilities(creatures, "All creatures lose all abilities.")],
    ..CardDef::BLANK
};
static GIVER: CardDef = CardDef {
    name: "Assignment Test Giver",
    types: TypeSet::of(&[CardType::Enchantment]),
    statics: &[static_ability(Layer::Ability, creatures, give_by_toughness, "All creatures have \"This creature assigns combat damage equal to its toughness rather than its power.\"")],
    ..CardDef::BLANK
};
static LAND_SETTER: CardDef = CardDef {
    name: "Assignment Test Land Setter",
    types: TypeSet::of(&[CardType::Enchantment]),
    statics: &[sets_land_subtypes(everything, set_plains, "All lands are Plains.")],
    ..CardDef::BLANK
};

fn gaining_game() -> Game {
    let table: &'static CardTable = Box::leak(Box::new(CardTable::new([&PLAIN, &PRINTED, &PRINTED_LAND, &STRIPPER, &GIVER, &LAND_SETTER, &ABSOLUTE, &TOUGHNESS])));
    Game::new(table, &crate::Setup { decks: vec![Vec::new(), Vec::new()], ..Default::default() })
}

/// `def` enters now, under `owner`, with a timestamp of its own.
fn enter(game: &mut Game, def: &'static CardDef, owner: PlayerId) -> ObjectId {
    let id = game.state.push_obj(Obj::new(game.cards.id_of(def), owner, Zone::BATTLEFIELD));
    let entered = game.state.timestamp();
    game.state.update_obj(id, |o| o.timestamp = entered);
    game.state.zone_push(Zone::BATTLEFIELD, id);
    if !def.statics.is_empty() {
        game.state.see_statics();
    }
    id
}

fn effect(game: &mut Game, on: ObjectId, kind: EffectKind) {
    cantrip_api::Game::add_effect(&mut cx(game), on, on, kind, Duration::EndOfTurn);
}

fn assigned(game: &mut Game, creature: ObjectId) -> u32 {
    static_rules::combat_damage_of(&cx(game), creature)
}

fn cleanup(game: &mut Game) {
    game.state.update_turn(|t| t.set_step(Step::Cleanup));
    crate::turn::begin_step(&mut cx(game)).expect("nothing to choose");
}

/// Streetwise Negotiator's rulings, CR 510.1a: the creature that gains the ability assigns its toughness, and no other
/// creature does. Its power is what it was, and at cleanup the ability ends with its effect (CR 514.2).
#[test]
fn a_gained_ability_assigns_toughness_changes_no_power_and_ends_with_its_duration() {
    let mut game = gaining_game();
    let gainer = enter(&mut game, &PLAIN, ME);
    let other = enter(&mut game, &PLAIN, ME);
    assert_eq!(assigned(&mut game, gainer), 1);
    effect(&mut game, gainer, gains());
    assert_eq!(assigned(&mut game, gainer), 3);
    assert_eq!(assigned(&mut game, other), 1, "only the creature that gained it");
    assert_eq!(cx(&mut game).chars(gainer).power, Some(1));
    cleanup(&mut game);
    assert_eq!(assigned(&mut game, gainer), 1, "it ended with the turn");
}

/// CR 510.1a: unlike "with toughness greater than its power", the ability has no such condition: a greater power goes
/// unused, and a toughness of 0 or less assigns nothing.
#[test]
fn a_gained_ability_reads_toughness_whatever_it_is() {
    for (power, toughness, damage) in [(5, 2, 2), (5, 0, 0), (5, -2, 0), (-2, 3, 3)] {
        let mut game = gaining_game();
        let creature = enter(&mut game, &PLAIN, ME);
        let card = game.cards.id_of(&PLAIN);
        game.state.update_obj(creature, |o| o.copy = Some(Copiable::of(card).sized(power, toughness)));
        effect(&mut game, creature, gains());
        assert_eq!(assigned(&mut game, creature), damage, "{power}/{toughness}");
    }
}

/// CR 613.1f, 613.7: abilities come and go in layer 6 in timestamp order. A grant before a loss is taken and a grant after
/// it is kept, whether a resolved effect or a static ability makes the loss.
#[test]
fn a_gained_ability_follows_the_timestamp_order_of_losing_all_abilities() {
    let mut game = gaining_game();
    let creature = enter(&mut game, &PLAIN, ME);
    effect(&mut game, creature, gains());
    effect(&mut game, creature, EffectKind::LoseAllAbilities);
    assert_eq!(assigned(&mut game, creature), 1, "granted, then lost");
    effect(&mut game, creature, gains());
    assert_eq!(assigned(&mut game, creature), 3, "granted again after the loss");

    let mut game = gaining_game();
    let creature = enter(&mut game, &PLAIN, ME);
    effect(&mut game, creature, gains());
    enter(&mut game, &STRIPPER, THEM);
    assert_eq!(assigned(&mut game, creature), 1, "granted, then a static ability took it");

    let mut game = gaining_game();
    let creature = enter(&mut game, &PLAIN, ME);
    enter(&mut game, &STRIPPER, THEM);
    effect(&mut game, creature, gains());
    assert_eq!(assigned(&mut game, creature), 3, "a grant timestamped after the static ability is kept");
}

/// CR 613.1f, 613.7: the same ability given by a static ability is in timestamp order with a loss, and is gone while its
/// giver has no abilities.
#[test]
fn an_ability_a_static_ability_gives_goes_with_a_later_loss_and_with_its_giver() {
    let mut game = gaining_game();
    let creature = enter(&mut game, &PLAIN, ME);
    let giver = enter(&mut game, &GIVER, ME);
    assert_eq!(assigned(&mut game, creature), 3);
    effect(&mut game, giver, EffectKind::LoseAllAbilities);
    assert_eq!(assigned(&mut game, creature), 1, "the giver has no abilities");

    let mut game = gaining_game();
    let creature = enter(&mut game, &PLAIN, ME);
    enter(&mut game, &GIVER, ME);
    effect(&mut game, creature, EffectKind::LoseAllAbilities);
    assert_eq!(assigned(&mut game, creature), 1, "lost after the giver's timestamp");

    let mut game = gaining_game();
    let creature = enter(&mut game, &PLAIN, ME);
    enter(&mut game, &STRIPPER, THEM);
    enter(&mut game, &GIVER, ME);
    assert_eq!(assigned(&mut game, creature), 3, "the giver is newer than the static loss");
}

/// CR 613.1f, 613.7: a creature's printed ability goes with its other abilities, by an effect or by a static ability; a
/// grant timestamped after the loss gives it the ability again, and only that grant holds it there.
#[test]
fn a_printed_ability_goes_with_a_loss_and_a_later_grant_is_kept() {
    let mut game = gaining_game();
    let printed = enter(&mut game, &PRINTED, ME);
    assert_eq!(assigned(&mut game, printed), 3);
    effect(&mut game, printed, EffectKind::LoseAllAbilities);
    assert_eq!(assigned(&mut game, printed), 1, "printed, then lost");
    effect(&mut game, printed, gains());
    assert_eq!(assigned(&mut game, printed), 3, "granted after the loss");
    cleanup(&mut game);
    assert_eq!(assigned(&mut game, printed), 3, "both ended: it has its printed ability again");

    let mut game = gaining_game();
    let printed = enter(&mut game, &PRINTED, ME);
    enter(&mut game, &STRIPPER, THEM);
    assert_eq!(assigned(&mut game, printed), 1, "a static ability took it");
    effect(&mut game, printed, gains());
    assert_eq!(assigned(&mut game, printed), 3, "granted after the static ability");
    cleanup(&mut game);
    assert_eq!(assigned(&mut game, printed), 1, "the grant ended, and the static ability still takes the printed one");
}

/// CR 305.7, 613.1d-f: a land type set in layer 4 takes what the land's rules text gives it, and an ability granted in layer
/// 6 comes after it whatever the timestamps say.
#[test]
fn a_land_type_takes_the_printed_ability_and_never_a_grant() {
    let mut game = gaining_game();
    let land = enter(&mut game, &PRINTED_LAND, ME);
    assert_eq!(assigned(&mut game, land), 3);
    enter(&mut game, &LAND_SETTER, THEM);
    assert_eq!(assigned(&mut game, land), 1, "the land type took the printed ability");

    let mut game = gaining_game();
    let land = enter(&mut game, &PRINTED_LAND, ME);
    effect(&mut game, land, gains());
    enter(&mut game, &LAND_SETTER, THEM);
    assert_eq!(assigned(&mut game, land), 3, "the grant is older than the land type, and stays");
}

/// CR 707.2, 613.1f: a copy has what its copiable values print, and nothing its original gained.
#[test]
fn a_copy_has_the_printed_ability_and_never_a_gained_one() {
    let mut game = gaining_game();
    let original = enter(&mut game, &PLAIN, ME);
    effect(&mut game, original, gains());
    let copy = enter(&mut game, &PLAIN, ME);
    let card = game.cards.id_of(&PLAIN);
    cantrip_api::Game::set_copy(&mut cx(&mut game), copy, Copiable::of(card));
    assert_eq!(assigned(&mut game, copy), 1, "a grant is no copiable value");
    let printed = game.cards.id_of(&PRINTED);
    cantrip_api::Game::set_copy(&mut cx(&mut game), copy, Copiable::of(printed));
    assert_eq!(assigned(&mut game, copy), 3, "what it copies prints the ability");
}

/// CR 510.1a-d: the gained ability is read unblocked, blocked and blocking, and the creature's power is unchanged.
#[test]
fn a_gained_ability_is_used_for_unblocked_attackers_blocked_attackers_and_blockers() {
    for arrangement in 0..3 {
        let blocking = arrangement == 2;
        let mut game = gaining_game();
        let gainer = enter(&mut game, &PLAIN, if blocking { THEM } else { ME });
        let other = enter(&mut game, &PLAIN, if blocking { ME } else { THEM });
        effect(&mut game, gainer, gains());
        let (attacker, blocker) = if blocking { (other, gainer) } else { (gainer, other) };
        game.state.set_attackers(vec![Attacker { blocked: arrangement > 0, ..Attacker::new(attacker, Target::Player(THEM), THEM, TypeSet::NONE) }]);
        if arrangement > 0 {
            game.state.set_blocks(vec![Block { blocker, attacker, first_strike: false }]);
        }
        game.state.update_turn(|t| t.set_step(Step::CombatDamage));
        crate::turn::begin_step(&mut cx(&mut game)).expect("no assignment choice");
        assert_eq!(game.state.player(THEM).life, if arrangement == 0 { 17 } else { 20 });
        assert_eq!(game.state.obj(other).damage, if arrangement == 0 { 0 } else { 3 });
        assert_eq!(game.state.obj(gainer).damage, if arrangement == 0 { 0 } else { 1 });
        assert_eq!(cx(&mut game).chars(gainer).power, Some(1));
    }
}

/// Streetwise Negotiator's ruling: "if [it] fights, it uses its actual power to determine how much damage it deals".
#[test]
fn a_creature_that_gained_the_ability_fights_with_its_power() {
    let mut game = gaining_game();
    let gainer = enter(&mut game, &PLAIN, ME);
    let other = enter(&mut game, &PLAIN, THEM);
    effect(&mut game, gainer, gains());
    cantrip_api::Game::fight(&mut cx(&mut game), gainer, other);
    assert_eq!(game.state.obj(other).damage, 1);
    assert_eq!(game.state.obj(gainer).damage, 1);
}

/// `attacker` attacks `THEM`, blocked by `blockers`, and the combat damage step is next.
fn blocked_by(game: &mut Game, attacker: ObjectId, blockers: &[ObjectId]) {
    game.state.set_attackers(vec![Attacker { blocked: true, ..Attacker::new(attacker, Target::Player(THEM), THEM, TypeSet::NONE) }]);
    game.state.set_blocks(blockers.iter().map(|&blocker| Block { blocker, attacker, first_strike: false }).collect());
    game.state.update_turn(|t| t.set_step(Step::CombatDamage));
}

/// The combat damage step with one division to make: the division it asks for, made as the engine suggests and dealt.
fn deal_the_suggested_division(game: &mut Game) -> DamageDivision {
    let frame = game.state.agenda.len();
    game.state.frame_push(cantrip_state::Frame::new(cantrip_state::FrameKind::BeginStep));
    let mut cx = cx(game);
    cx.frame = frame;
    assert!(matches!(crate::turn::begin_step(&mut cx), Err(crate::Stop::Suspend)));
    let request = cx.s().agenda[frame].pending.clone().expect("a combat assignment");
    let RequestKind::AssignCombatDamage { divisions } = &request.kind else { panic!("combat assignment expected") };
    assert_eq!(divisions.len(), 1);
    let answer = Answer::Shares(DamageDivision::suggested_shares(divisions));
    crate::check_answer(cx.g, &request, &answer).expect("the suggested division is legal");
    cx.st().update_frame(frame, |f| {
        f.pending = None;
        f.answers.push((None, answer));
    });
    crate::turn::begin_step(&mut cx).expect("the assignment was answered");
    divisions[0].clone()
}

/// CR 702.19b, 702.2b: trample divides the toughness it assigns, not its power: with lethal already marked but for 1, or with
/// deathtouch making 1 lethal, what is left of 3 reaches the player, where a power of 1 would have no division at all.
#[test]
fn trample_and_deathtouch_divide_the_toughness_assigned() {
    for deathtouch in [false, true] {
        let mut game = gaining_game();
        let gainer = enter(&mut game, &PLAIN, ME);
        let blocker = enter(&mut game, &PLAIN, THEM);
        effect(&mut game, gainer, gains());
        effect(&mut game, gainer, EffectKind::AddKeywords(Keywords::of(&[Keyword::Trample])));
        if deathtouch {
            effect(&mut game, gainer, EffectKind::AddKeywords(Keywords::of(&[Keyword::Deathtouch])));
        } else {
            game.state.update_obj(blocker, |o| o.damage = 2);
        }
        blocked_by(&mut game, gainer, &[blocker]);
        let division = deal_the_suggested_division(&mut game);
        assert_eq!(division.damage, 3);
        assert_eq!(division.lethal, [1]);
        assert_eq!((division.suggested.as_slice(), division.suggested_to_defender), ([1].as_slice(), 2));
        assert_eq!(game.state.player(THEM).life, 18);
    }
}

/// CR 510.1c: blocked by two creatures it divides its toughness among them, and each of them deals its own power back.
#[test]
fn multiple_blockers_share_the_toughness_assigned() {
    let mut game = gaining_game();
    let gainer = enter(&mut game, &PLAIN, ME);
    let (first, second) = (enter(&mut game, &PLAIN, THEM), enter(&mut game, &PLAIN, THEM));
    effect(&mut game, gainer, gains());
    blocked_by(&mut game, gainer, &[first, second]);
    let division = deal_the_suggested_division(&mut game);
    assert_eq!(division.damage, 3);
    assert_eq!(division.lethal, [3, 3]);
    assert_eq!(game.state.obj(first).damage + game.state.obj(second).damage, 3);
    assert_eq!(game.state.obj(gainer).damage, 2);
}

/// CR 702.4b, 702.7b, 510.1a: each combat damage step reads the toughness it has then: double strike assigns in both steps, a
/// greater toughness the second time, and first strike in the first alone.
#[test]
fn each_combat_damage_step_reads_toughness_for_double_and_first_strike() {
    for (strike, life) in [(Keyword::DoubleStrike, 12), (Keyword::FirstStrike, 17)] {
        let mut game = gaining_game();
        let gainer = enter(&mut game, &PLAIN, ME);
        effect(&mut game, gainer, gains());
        effect(&mut game, gainer, EffectKind::AddKeywords(Keywords::of(&[strike])));
        game.state.set_attackers(vec![Attacker::new(gainer, Target::Player(THEM), THEM, TypeSet::NONE)]);
        game.state.update_turn(|t| t.set_step(Step::CombatDamage));
        crate::turn::begin_step(&mut cx(&mut game)).expect("no assignment choice");
        assert_eq!(game.state.player(THEM).life, 17);
        let card = game.cards.id_of(&PLAIN);
        game.state.update_obj(gainer, |o| o.copy = Some(Copiable::of(card).sized(1, 5)));
        crate::turn::begin_step(&mut cx(&mut game)).expect("no assignment choice");
        assert_eq!(game.state.player(THEM).life, life);
    }
}

/// CR 510.1a, 609.4: the ability has no condition, so The Kingpin of Crime's "with toughness greater than their power" does not
/// hold it back, and it says toughness where Loot, the Anomaly's rule says its negative power's magnitude. When the ability is
/// lost, what another permanent or an effect says goes on being read (CR 613.1f).
#[test]
fn the_ability_beside_the_other_assignment_rules() {
    let sized = |game: &mut Game, id: ObjectId, def: &'static CardDef, power, toughness| {
        let card = game.cards.id_of(def);
        game.state.update_obj(id, |o| o.copy = Some(Copiable::of(card).sized(power, toughness)));
    };
    let mut game = gaining_game();
    let creature = enter(&mut game, &PLAIN, ME);
    sized(&mut game, creature, &PLAIN, 5, 2);
    cantrip_api::Game::add_player_effect(&mut cx(&mut game), ME, creature, EffectKind::CreaturesAssignDamageByToughness, Duration::EndOfTurn);
    assert_eq!(assigned(&mut game, creature), 5, "the effect's own condition does not hold");
    effect(&mut game, creature, gains());
    assert_eq!(assigned(&mut game, creature), 2, "the ability has no such condition");

    let mut game = gaining_game();
    let creature = enter(&mut game, &ABSOLUTE, ME);
    assert_eq!(assigned(&mut game, creature), 2, "negative power's magnitude");
    effect(&mut game, creature, gains());
    assert_eq!(assigned(&mut game, creature), 4, "toughness, never the exception about power");

    let mut game = gaining_game();
    let creature = enter(&mut game, &PLAIN, ME);
    effect(&mut game, creature, gains());
    effect(&mut game, creature, EffectKind::LoseAllAbilities);
    cantrip_api::Game::add_player_effect(&mut cx(&mut game), ME, creature, EffectKind::CreaturesAssignDamageByToughness, Duration::EndOfTurn);
    assert_eq!(assigned(&mut game, creature), 3, "an effect about the player, which the loss does not take");

    let mut game = gaining_game();
    let creature = enter(&mut game, &PLAIN, ME);
    effect(&mut game, creature, gains());
    effect(&mut game, creature, EffectKind::LoseAllAbilities);
    enter(&mut game, &TOUGHNESS, ME);
    assert_eq!(assigned(&mut game, creature), 3, "another permanent's rule, which the loss does not take");
}
