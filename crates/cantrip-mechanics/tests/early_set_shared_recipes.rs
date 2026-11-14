//! Forced positions for the recipes shared by the early-set card groups.
use cantrip_api::*;
use cantrip_mechanics::{attach, effects, mana};
use cantrip_rules::{Game as RulesGame, Setup};
use cantrip_state::{Frame, FrameKind, Span, INLINE_TARGETS};

const YOU: PlayerId = PlayerId(0);
const THEM: PlayerId = PlayerId(1);
static CREATURE: CardDef = CardDef {
    name: "Recipe creature",
    types: TypeSet::of(&[CardType::Creature]),
    power: Some(1),
    toughness: Some(2),
    ..CardDef::BLANK
};
static AURA: CardDef = CardDef {
    name: "Recipe Aura",
    types: TypeSet::of(&[CardType::Enchantment]),
    subtypes: SubtypeSet::of(&[Subtype::Aura]),
    enchant: Some(cantrip_mechanics::targets::CREATURE),
    targeting_rules: &[attach::ENCHANTED_CANT_BE_TARGET_OF_SPELLS],
    ..CardDef::BLANK
};
static SPELL: CardDef = CardDef {
    name: "Recipe spell",
    types: TypeSet::of(&[CardType::Instant]),
    ..CardDef::BLANK
};

fn creature(g: &dyn Game, player: PlayerId) -> ObjectId {
    g.state()
        .battlefield()
        .iter()
        .copied()
        .find(|&id| g.obj(id).card == g.card_id(&CREATURE) && g.controller(id) == player)
        .unwrap()
}

fn aura_probe(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let aura = g
        .state()
        .battlefield()
        .iter()
        .copied()
        .find(|&id| g.obj(id).card == g.card_id(&AURA))
        .unwrap();
    let own = creature(g, YOU);
    let opposing = creature(g, THEM);
    let spell = g.state().zone(Zone::STACK)[0];
    let spell = Asker {
        spell: true,
        ..Asker::new(spell, THEM)
    };
    let ability = Asker::new(item.object, THEM);
    assert_eq!(g.obj(aura).attached_to, Some(own));
    assert!(!g.can_target(&spell, Target::Object(own)));
    assert!(
        g.can_target(&ability, Target::Object(own)),
        "this is not shroud"
    );
    assert!(g.can_target(&spell, Target::Object(opposing)));
    g.add_effect(
        item.object,
        own,
        EffectKind::LoseAllAbilities,
        Duration::EndOfTurn,
    );
    g.end_sentence();
    assert!(
        !g.can_target(&spell, Target::Object(own)),
        "the Aura supplies the rule, not its host"
    );
    assert!(g.attach(aura, opposing));
    assert!(
        g.can_target(&spell, Target::Object(own)),
        "moving the Aura releases its old host"
    );
    assert!(!g.can_target(&spell, Target::Object(opposing)));
    assert!(g.can_target(&ability, Target::Object(opposing)));
    g.sacrifice(YOU, &[aura]);
    g.end_sentence();
    assert!(
        g.can_target(&spell, Target::Object(opposing)),
        "the rule ends when the Aura leaves"
    );
    Ok(())
}

fn counters_probe(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    effects::counters_on_self::<{ CounterKind::Doom as usize }, 1>(g, item)?;
    effects::counters_on_self::<{ CounterKind::Time as usize }, 1>(g, item)?;
    effects::charge_counters_on_self::<4>(g, item)?;
    assert_eq!(g.obj(item.object).counters(CounterKind::Doom), 1);
    assert_eq!(g.obj(item.object).counters(CounterKind::Time), 1);
    assert_eq!(g.obj(item.object).counters(CounterKind::Charge), 4);
    let moved = g.sacrifice(YOU, &[item.object]);
    g.end_sentence();
    let new_object = moved.objects()[0];
    effects::counters_on_self::<{ CounterKind::Time as usize }, 1>(g, item)?;
    assert_eq!(
        g.obj(item.object).counters(CounterKind::Time),
        1,
        "the departed object's LKI stays unchanged"
    );
    assert_eq!(
        g.obj(new_object).counters(CounterKind::Time),
        0,
        "a resolving ability cannot put counters on the new object"
    );
    Ok(())
}

fn life_probe(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let sacrificed = creature(g, YOU);
    g.add_effect(
        item.object,
        sacrificed,
        EffectKind::Pump(0, 3),
        Duration::EndOfTurn,
    );
    g.end_sentence();
    assert_eq!(g.chars(sacrificed).toughness, Some(5));
    let moved = g.sacrifice(YOU, &[sacrificed]);
    g.end_sentence();
    assert_eq!(g.chars(moved.objects()[0]).toughness, Some(2));
    let mut paid = *item;
    paid.kind = StackKind::Activated;
    paid.controller = THEM;
    let before = g.state().player(THEM).life;
    let owner_before = g.state().player(YOU).life;
    effects::gain_life_equal_sacrificed_creatures_toughness(g, &paid)?;
    assert_eq!(
        g.state().player(THEM).life,
        before + 5,
        "use the paid object's battlefield toughness and the ability's controller"
    );
    assert_eq!(g.state().player(YOU).life, owner_before);
    Ok(())
}

fn mana_probe(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let original = ManaProduction {
        permanent: item.object,
        player: YOU,
        to: THEM,
        mana: ManaPool([1, 2, 0, 0, 3, 4]),
        restriction: Some(ManaRestriction::Spells(TypeSet::of(&[CardType::Creature]))),
        lasts: ManaLasts::Turn,
    };
    let mut proposal = Proposal::ManaProduction(original);
    mana::produce_colorless_instead(g, &Asker::new(item.object, YOU), &mut proposal);
    assert_eq!(
        proposal,
        Proposal::ManaProduction(ManaProduction {
            mana: ManaPool([0, 0, 0, 0, 0, 10]),
            ..original
        }),
        "only mana types change; the tapper, recipient, restriction and lifetime stay intact"
    );
    let once = proposal;
    mana::produce_colorless_instead(g, &Asker::new(item.object, YOU), &mut proposal);
    assert_eq!(proposal, once, "already-colorless mana is unchanged");
    Ok(())
}

static AURA_PROBE: CardDef = CardDef {
    name: "Aura recipe probe",
    delayed: &[delayed_trigger(aura_probe, "Probe Aura rule")],
    ..COUNTERS_PROBE
};
static COUNTERS_PROBE: CardDef = CardDef {
    name: "Counters recipe probe",
    types: TypeSet::of(&[CardType::Artifact]),
    delayed: &[delayed_trigger(counters_probe, "Probe counters")],
    ..CardDef::BLANK
};
static LIFE_PROBE: CardDef = CardDef {
    name: "Life recipe probe",
    delayed: &[delayed_trigger(life_probe, "Probe LKI life")],
    ..COUNTERS_PROBE
};
static MANA_PROBE: CardDef = CardDef {
    name: "Mana recipe probe",
    delayed: &[delayed_trigger(mana_probe, "Probe mana types")],
    ..COUNTERS_PROBE
};

fn run(probe: &'static CardDef) {
    let cards = Box::leak(Box::new(CardTable::new([probe, &CREATURE, &AURA, &SPELL])));
    let mut game = RulesGame::new(
        cards,
        &Setup {
            decks: vec![vec![], vec![]],
            ..Default::default()
        },
    );
    game.verify = true;
    let mut put = |def, owner, zone| {
        let id = game.state.push_obj(Obj::new(cards.id_of(def), owner, zone));
        game.state.zone_push(zone, id);
        id
    };
    let source = put(probe, YOU, Zone::BATTLEFIELD);
    let sacrificed = put(&CREATURE, YOU, Zone::BATTLEFIELD);
    put(&CREATURE, THEM, Zone::BATTLEFIELD);
    let aura = put(&AURA, YOU, Zone::BATTLEFIELD);
    // A legal initial attachment, before the resolving probe deliberately moves it.
    game.state
        .update_obj(aura, |obj| obj.attached_to = Some(sacrificed));
    let spell = game
        .state
        .push_obj(Obj::new(cards.id_of(&SPELL), THEM, Zone::STACK));
    game.state.zone_push(Zone::STACK, spell);
    let members = game.state.members_push(&[sacrificed]);
    game.state.stack_push(StackItem {
        target_choosers: Span::default(),
        instance: AbilityAcquisition::OWN,
        event_amount: None,
        kind: StackKind::Triggered,
        object: source,
        ability: AbilityRef::delayed(cards.id_of(probe), 0),
        controller: YOU,
        inline_targets: [Target::None; INLINE_TARGETS],
        target_count: 0,
        inline_spec_of: [0; INLINE_TARGETS],
        x: 0,
        kicked: 0,
        counted: 0,
        paid: 0,
        modes: 0,
        way: 0,
        choice: 0,
        subject: Target::None,
        data: [0, 0],
        members,
        serial: 0,
        gift: None,
        shares: StackItem::NO_SHARES,
        spill: 0,
        exile_instead: ExileInstead::Never,
        flashback: false,
        attacking: false,
        since: 0,
        tapped_since: 0,
        copied: false,
        cast_known: u32::MAX,
        acquired: Acquired::OWN,
        words: Words::NONE,
    });
    let serial = game.state.stack.last().unwrap().serial;
    game.state
        .frame_push(Frame::new(FrameKind::Resolve { serial }));
    game.run();
}

#[test]
fn aura_spell_restriction_follows_attachment_without_becoming_shroud() {
    run(&AURA_PROBE);
}
#[test]
fn self_counter_callbacks_do_not_follow_a_departed_source() {
    run(&COUNTERS_PROBE);
}
#[test]
fn sacrifice_life_uses_last_known_toughness_and_ability_controller() {
    run(&LIFE_PROBE);
}
#[test]
fn colorless_replacement_preserves_production_metadata_and_total() {
    run(&MANA_PROBE);
}
