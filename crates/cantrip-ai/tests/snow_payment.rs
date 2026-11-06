//! Every legal snow allocation remains a policy choice, including under legacy model input adaptation.
use cantrip_ai::{Answering, Knowledge, observe_indexed};
use cantrip_engine::*;
use cantrip_obs::{InfoMode, Ref};
use cantrip_obs::encode::{self, Origin};
use cantrip_obs::factor::{FactoredAnswer, Machine, Tok};
const YOU: PlayerId = PlayerId(0);
static SOURCE: CardDef = CardDef { name: "AI snow source", supertypes: Supertypes { snow: true, basic: false, legendary: false }, types: TypeSet::of(&[CardType::Land]), subtypes: SubtypeSet::of(&[Subtype::Island]), ..CardDef::BLANK };
static SPELL: CardDef = CardDef { name: "AI snow spell", cost: mana("{U}{S}"), has_cost: true, types: TypeSet::of(&[CardType::Creature]), power: Some(1), toughness: Some(1), ..CardDef::BLANK };
fn position(pool: bool) -> Game {
    let table = Box::leak(Box::new(CardTable::new([&SOURCE, &SPELL])));
    let mut g = Game::new(table, &Setup { decks: vec![vec![table.id_of(&SOURCE); 20]; 2], ..Default::default() });
    while !g.state.agenda.is_empty() { g.state.frame_remove(0); }
    g.state.update_turn(|t| { t.active = YOU; t.priority = YOU; t.set_step(Step::PrecombatMain); });
    g.state.frame_push(Frame::new(FrameKind::Priority));
    let source = g.state.push_obj(Obj::new(table.id_of(&SOURCE), YOU, Zone::BATTLEFIELD)); g.state.zone_push(Zone::BATTLEFIELD, source);
    let spell = g.state.push_obj(Obj::new(table.id_of(&SPELL), YOU, Zone::hand(YOU))); g.state.zone_push(Zone::hand(YOU), spell);
    if pool {
        g.state.add_mana(YOU, Mana::U, 2, source, None, ManaLasts::Step, true);
        g.state.add_mana(YOU, Mana::R, 1, source, None, ManaLasts::Step, true);
    } else {
        let second = g.state.push_obj(Obj::new(table.id_of(&SOURCE), YOU, Zone::BATTLEFIELD)); g.state.zone_push(Zone::BATTLEFIELD, second);
    }
    g.run(); let r = g.pending().unwrap().clone();
    let RequestKind::Priority { options } = &r.kind else { panic!("priority") };
    let a = Action::of(options.iter().find(|o| o.kind == ActionKind::CastSpell).unwrap());
    g.answer(r.id, YOU, Answer::Act(a)).unwrap(); g.run();
    if pool {
        let window = g.pending().unwrap().clone();
        let RequestKind::PayCosts { options, mana_abilities: true, .. } = &window.kind else {
            panic!("the casting mana-ability window: {:?}", window.kind)
        };
        let close = Action::of(options.iter().find(|o| o.kind == ActionKind::ConfirmCasting).unwrap());
        g.answer(window.id, YOU, Answer::Act(close)).unwrap();
        g.run();
    }
    g
}
#[test]
fn every_snow_assignment_is_factored_encoded_and_maps_back_exactly() {
    let g = position(true); let r = g.pending().unwrap();
    let (obs, index) = observe_indexed(&g, &Knowledge::of(&g, YOU), YOU, InfoMode::Full).unwrap();
    assert_eq!(obs.players[0].snow_pool, [0, 2, 0, 1, 0, 0]);
    let adapter = Answering::new(&g, r, &obs, &index);
    assert!(cantrip_obs::factor::snow_payment(&adapter.ask.kind));
    let machine = Machine::new(&adapter.ask, &[]);
    let step = machine.step().unwrap();
    assert!(step.tokens.iter().any(|t| t.kind == Tok::Option));
    assert!(!step.tokens.iter().any(|t| t.kind == Tok::AutoPay));
    let encoded = encode::encode(&obs, step, &[], &[], cantrip_ai::record::table());
    assert_eq!(encoded.nums_of(0)[encode::game::SNOW_COST], 1.0 / 8.0);
    let p = encoded.origin.iter().position(|o| *o == Origin::Player(0)).unwrap();
    assert_eq!(encoded.nums_of(p)[encode::player::SNOW_POOL + 1], 2.0 / 8.0);
    let mut flags = Vec::new();
    for token in step.tokens.iter().filter(|t| t.kind == Tok::Option) {
        let factored = FactoredAnswer::Act { option: token.n, targets: vec![], color: None };
        let answer = adapter.engine(&g, r, &factored).unwrap();
        if let Answer::Act(a) = &answer { if a.kind == ActionKind::AssignMana {
            flags.push((a.mana, a.snow, a.pay_snow));
            assert_eq!(adapter.factored(r, &answer, None), Some(factored.clone()));
            let decomposed = cantrip_obs::factor::decompose(&adapter.ask, &[], &factored).unwrap();
            assert_eq!(decomposed.first(), Some(token));
            let mut m = Machine::new(&adapter.ask, &[]); assert!(m.choose_token(*token));
            assert_eq!(m.answer(), Some(&factored));
            let t = encoded.origin.iter().position(|o| *o == Origin::Option(step.tokens.iter().position(|t| t == token).unwrap())).unwrap();
            assert_eq!(encoded.nums_of(t)[encode::option::SNOW], 1.0);
            assert_eq!(encoded.nums_of(t)[encode::option::PAY_SNOW], a.pay_snow as u8 as f32);
        }}
    }
    assert!(flags.contains(&(Some(Mana::U), true, true)));
    assert!(flags.contains(&(Some(Mana::U), true, false)));
    assert!(flags.contains(&(Some(Mana::R), true, true)));
    assert!(!flags.contains(&(Some(Mana::R), true, false)));
    // No policy-independent AutoPay shortcut replaces these decisions.
    assert!(adapter.engine(&g, r, &FactoredAnswer::AutoPay).is_err());
    assert!(step.tokens.iter().any(|t| t.a == Ref::None));
}

#[test]
fn snow_production_choices_are_not_pruned_by_the_ordinary_autotapper() {
    let g = position(false); let r = g.pending().unwrap();
    assert!(!cantrip_ai::atomic::unpayable(&g, r));
    let filtered = cantrip_ai::atomic::filter(&g, r);
    assert_eq!(format!("{:?}", filtered), format!("{:?}", r));
    let (obs, index) = observe_indexed(&g, &Knowledge::of(&g, YOU), YOU, InfoMode::Full).unwrap();
    let adapter = Answering::new(&g, r, &obs, &index);
    let machine = Machine::new(&adapter.ask, &[]);
    let mut activations = 0;
    for token in machine.step().unwrap().tokens.iter().filter(|t| t.kind == Tok::Option) {
        let factored = FactoredAnswer::Act { option: token.n, targets: vec![], color: None };
        let answer = adapter.engine(&g, r, &factored).unwrap();
        if matches!(answer, Answer::Act(Action { kind: ActionKind::ActivateAbility, .. })) { activations += 1; }
    }
    assert_eq!(activations, 2);
}

#[test]
fn snow_observation_caps_aggregate_across_distinct_restricted_pools() {
    use cantrip_ai::eval::{FeatureBatch, InputCompatibility};
    let mut g = position(false);
    let source = g.state.battlefield()[0];
    for restriction in [ManaRestriction::ARTIFACT_SPELLS, ManaRestriction::ABILITIES] {
        g.state.add_mana(YOU, Mana::G, 1 << 31, source, Some(restriction), ManaLasts::Step, true);
    }
    let total: u64 = g.state.mana_from_of(YOU).filter(|row| row.snow).map(|row| u64::from(row.pool.get(Mana::G))).sum();
    assert_eq!(total, 1u64 << 32, "both restricted pools retain their accepted native quantities");
    let r = g.pending().unwrap();
    let (obs, index) = observe_indexed(&g, &Knowledge::of(&g, YOU), YOU, InfoMode::Full).unwrap();
    assert_eq!(obs.players[0].snow_pool[Mana::G as usize], u32::MAX, "snow aggregation saturates at the bounded mana domain");
    let json = serde_json::to_value(&obs).unwrap();
    assert_eq!(json["players"][0]["snow_pool"][Mana::G as usize].as_u64(), Some(u64::from(u32::MAX)));
    let roundtrip: cantrip_obs::Observation = serde_json::from_str(&serde_json::to_string(&obs).unwrap()).unwrap();
    assert_eq!(roundtrip, obs, "the capped aggregate roundtrips exactly");

    let adapter = Answering::new(&g, r, &roundtrip, &index);
    let machine = Machine::new(&adapter.ask, &[]);
    let encoded = encode::encode(&roundtrip, machine.step().unwrap(), &[], &[], cantrip_ai::record::table());
    let player = encoded.origin.iter().position(|o| *o == Origin::Player(0)).unwrap();
    let feature = encode::player::SNOW_POOL + Mana::G as usize;
    assert_eq!(encoded.nums_of(player)[feature], 65504.0, "the large aggregate uses the encoder's finite f16 bound instead of wrapping to zero");
    let compact = encode::from_compact(&encode::to_compact(&encoded)).unwrap();
    assert_eq!(compact.nums, encoded.nums, "the bounded feature roundtrips through compact model input");
    let mut batch = FeatureBatch::new(&[&encoded]);
    let native = batch.nums.clone();
    InputCompatibility::Native.adapt(&mut batch);
    assert_eq!(batch.nums, native, "schema3 retains the encoded aggregate");
    InputCompatibility::LegacySnowInputV2.adapt(&mut batch);
    assert_eq!(batch.nums[player * encode::NUMS + feature], 0.0);
    for (token, &kind) in batch.kind.iter().enumerate() {
        for column in 0..encode::NUMS {
            let at = token * encode::NUMS + column;
            if !InputCompatibility::LegacySnowInputV2.masks(kind as u8, column) {
                assert_eq!(batch.nums[at], native[at], "legacy adaptation preserves every existing feature");
            }
        }
    }
}
