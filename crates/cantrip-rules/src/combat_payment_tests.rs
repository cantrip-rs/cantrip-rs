//! Shared CR 508.1h-j payments: Flooded Woodlands, Reclamation and Leviathan.
use crate::{Cx, Game, Setup, static_rules};
use cantrip_api::{Game as Rules, *};
use cantrip_state::{Frame, FrameKind};
const A: PlayerId = PlayerId(0);
const B: PlayerId = PlayerId(1);
fn all(_: &dyn Query, _: &Asker, _: ObjectId, _: Target) -> bool { true }
fn this(_: &dyn Query, a: &Asker, id: ObjectId, _: Target) -> bool { id == a.source }
fn land(q: &dyn Query, _: &Asker, id: ObjectId) -> bool { q.chars(id).is(CardType::Land) }
fn island(q: &dyn Query, _: &Asker, id: ObjectId) -> bool { q.chars(id).has_subtype(Subtype::Island) }
fn linked_land(q: &dyn Query, a: &Asker, id: ObjectId) -> bool { q.obj(a.source).attached_to == Some(id) }
static FILLER: CardDef = CardDef { name: "Combat payment filler", types: TypeSet::of(&[CardType::Land]), ..CardDef::BLANK };
static BEAR: CardDef = CardDef { name: "Combat payment attacker", types: TypeSet::of(&[CardType::Creature]), power: Some(2), toughness: Some(2), ..CardDef::BLANK };
static LAND: CardDef = CardDef { name: "Combat payment Island", subtypes: SubtypeSet::of(&[Subtype::Island]), ..FILLER };
static FOREST: CardDef = CardDef { name: "Combat payment Forest", subtypes: SubtypeSet::of(&[Subtype::Forest]), ..FILLER };
static TAX: CardDef = CardDef { name: "Combat payment land tax", types: TypeSet::of(&[CardType::Enchantment]), attack_cost_rules: &[AttackPayment { applies: all, cost: Cost::sacrifice(1, land) }], ..CardDef::BLANK };
static LEVIATHAN: CardDef = CardDef { name: "Combat payment two Islands", attack_cost_rules: &[AttackPayment { applies: this, cost: Cost::sacrifice(2, island) }], ..BEAR };
static LINKED: CardDef = CardDef { name: "Combat payment source-sensitive tax", attack_cost_rules: &[AttackPayment { applies: all, cost: Cost::sacrifice(1, linked_land) }], ..TAX };
static MIXED: CardDef = CardDef { name: "Combat payment mixed tax", attack_cost_rules: &[AttackPayment { applies: all, cost: Cost::sequence(&[Cost::sacrifice(1, land), Cost::sacrifice(1, island)]) }], ..TAX };
static MANA_TAX: CardDef = CardDef { name: "Combat payment removable mana tax", attack_cost_rules: &[AttackPayment { applies: all, cost: cost("{1}").and(Cost::sacrifice(1, land)) }], abilities: &[mana_ability(SACRIFICE_THIS, add(Mana::C), "Sacrifice this: Add {C}.")], ..TAX };
static LAND_CREATURE: CardDef = CardDef { name: "Combat payment land creature", types: TypeSet::of(&[CardType::Land, CardType::Creature]), ..BEAR };
fn noop(_: &mut dyn Rules, _: &StackItem) -> Result<(), Suspend> { Ok(()) }
static RECRUITER: CardDef = CardDef { name: "Combat payment enlist", abilities: &[paid_as_it_attacks(AttackOption::Enlist, noop, "Enlist")], ..BEAR };
fn game() -> Game {
    let cards = Box::leak(Box::new(CardTable::new([&FILLER,&BEAR,&LAND,&FOREST,&TAX,&LEVIATHAN,&LINKED,&MIXED,&MANA_TAX,&LAND_CREATURE,&RECRUITER])));
    let filler = cards.id_of(&FILLER);
    let mut g = Game::new(cards, &Setup { decks: vec![vec![filler;20];2], shuffle: false, ..Default::default() });
    g.verify = true;
    while !g.state.agenda.is_empty() { g.state.frame_remove(0); }
    g.state.frame_push(Frame::new(FrameKind::BeginStep));
    g.state.update_turn(|t| { t.number=2;t.active=A;t.priority=A;t.set_step(Step::DeclareAttackers); });
    g.state.update_player(A,|p|p.turn_began=1);
    g
}
fn put(g: &mut Game, d: &'static CardDef, p: PlayerId) -> ObjectId {
    let id = g.state.push_obj(Obj::new(g.cards.id_of(d),p,Zone::BATTLEFIELD));
    let ts=g.state.timestamp();g.state.update_obj(id,|o|{o.controlled_since=0;o.timestamp=ts;});
    g.state.zone_push(Zone::BATTLEFIELD,id);g.state.see_statics();id
}
fn cx(g: &mut Game) -> Cx<'_> { Cx { g,frame:0,cursor:0,mana_source:None,entering:None,lineage:vec![],damage:None,asked_at:None,doer:None,producing:None } }
fn next(g: &mut Game) -> Request {
    for _ in 0..100 { g.run();let r=g.pending().cloned().expect("question");match r.kind {
        RequestKind::Priority{..} => g.answer(r.id,r.player,Answer::Pass).unwrap(),
        RequestKind::OrderTriggers{ref sources,..} => g.answer(r.id,r.player,Answer::Order((0..sources.len() as u32).collect())).unwrap(),
        _ => return r,
    }}panic!("no question")
}
fn answer(g: &mut Game, r: Request, a: Answer) {g.answer(r.id,r.player,a).unwrap();}
fn attack(g: &mut Game, ids: &[ObjectId]) {let r=next(g);assert!(matches!(r.kind,RequestKind::DeclareAttackers{..}),"{:?}",r.kind);answer(g,r,Answer::Attack(ids.iter().map(|&id|(id,Target::Player(B))).collect()));}
fn choose(g: &mut Game, ids: &[ObjectId]) {let r=next(g);assert!(matches!(r.kind,RequestKind::ChooseCards {purpose:Purpose::SacrificeCost,..}),"{:?}",r.kind);answer(g,r,Answer::Cards(ids.to_vec()));}
fn done(g: &mut Game) {g.run();assert!(g.state.turn.step()>=Step::DeclareAttackers);assert_eq!(g.state.attackers.len(),g.events().iter().filter(|r|matches!(r.event,Event::Attacks{..})).count());}
#[test]
fn nonmana_costs_are_not_free_attacks_and_only_live_rules_apply() {
    let mut g=game();let a=put(&mut g,&BEAR,A);assert!(static_rules::attack_costs(&cx(&mut g),&[a],&[Target::Player(B)]).is_empty());
    let tax=put(&mut g,&TAX,B);let costs=static_rules::attack_costs(&cx(&mut g),&[a],&[Target::Player(B)]);
    assert_eq!(costs.len(),1);assert!(costs[0].cost.is_zero());assert!(!cantrip_state::AttackCost::free(&costs,a,Target::Player(B)));
    cx(&mut g).add_effect(tax,tax,EffectKind::LoseAllAbilities,Duration::EndOfTurn);
    assert!(static_rules::attack_costs(&cx(&mut g),&[a],&[Target::Player(B)]).is_empty());
}
#[test]
fn one_land_is_paid_for_each_attacker_with_separate_history_receipts() {
    let mut g=game();let tax=put(&mut g,&TAX,B);let ids=[put(&mut g,&BEAR,A),put(&mut g,&BEAR,A)];let lands=[put(&mut g,&LAND,A),put(&mut g,&FOREST,A)];
    attack(&mut g,&ids);choose(&mut g,&lands[..1]);choose(&mut g,&lands[1..]);done(&mut g);
    assert_eq!(g.state.attackers.len(),2);
    let receipts:Vec<_>=g.events().iter().filter_map(|r|match r.event {Event::CombatCostObjectPaid{source,attacker,object,..}=>Some((source,attacker,object)),_=>None}).collect();
    assert_eq!(receipts,vec![(tax,ids[0],lands[0]),(tax,ids[1],lands[1])]);
}
#[test]
fn leviathan_asks_for_two_islands_in_one_sacrifice() {
    let mut g=game();let a=put(&mut g,&LEVIATHAN,A);let lands=[put(&mut g,&LAND,A),put(&mut g,&LAND,A)];let forest=put(&mut g,&FOREST,A);
    attack(&mut g,&[a]);let r=next(&mut g);let RequestKind::ChooseCards{ref cards,ref rule,..}=r.kind else{panic!("{:?}",r.kind)};
    assert_eq!(rule.count,Count::Exactly(2));assert!(!cards.contains(&forest));answer(&mut g,r,Answer::Cards(lands.to_vec()));done(&mut g);assert_eq!(g.state.attackers.len(),1);
}
#[test]
fn a_partially_paid_declaration_rolls_back_sacrifices_taps_and_receipts() {
    let mut g=game();put(&mut g,&TAX,B);let ids=[put(&mut g,&BEAR,A),put(&mut g,&BEAR,A)];let l=put(&mut g,&LAND,A);
    attack(&mut g,&ids);choose(&mut g,&[l]);let r=next(&mut g);assert!(matches!(r.kind,RequestKind::DeclareAttackers{..}),"{:?}",r.kind);
    assert!(g.state.obj(l).zone.is(ZoneKind::Battlefield));assert!(!g.state.obj(ids[0]).tapped);assert!(!g.state.obj(ids[1]).tapped);
    assert!(!g.events().iter().any(|r|matches!(r.event,Event::CombatPaymentBegan{..}|Event::CombatCostObjectPaid{..}|Event::Attacks{..})));
    answer(&mut g,r,Answer::Attack(vec![(ids[0],Target::Player(B))]));choose(&mut g,&[l]);done(&mut g);assert_eq!(g.state.attackers.len(),1);
}
#[test]
fn overlapping_components_allow_the_player_to_choose_payment_order() {
    let mut g=game();put(&mut g,&MIXED,B);let a=put(&mut g,&BEAR,A);let i=put(&mut g,&LAND,A);let f=put(&mut g,&FOREST,A);
    attack(&mut g,&[a]);let r=next(&mut g);let RequestKind::ChooseCostPayment{ref choices,..}=r.kind else{panic!("{:?}",r.kind)};assert_eq!(choices.len(),2);
    answer(&mut g,r,Answer::CostPayment(1));choose(&mut g,&[i]);choose(&mut g,&[f]);done(&mut g);assert_eq!(g.state.attackers.len(),1);
}
#[test]
fn each_cost_filter_reads_its_own_rule_source() {
    let mut g=game();let first=put(&mut g,&LINKED,B);let second=put(&mut g,&LINKED,B);let a=put(&mut g,&BEAR,A);let l=put(&mut g,&LAND,A);let f=put(&mut g,&FOREST,A);
    g.state.update_obj(first,|o|o.attached_to=Some(l));g.state.update_obj(second,|o|o.attached_to=Some(f));g.state.see_statics();
    attack(&mut g,&[a]);choose(&mut g,&[l]);let r=next(&mut g);let RequestKind::ChooseCards{ref cards,..}=r.kind else{panic!("{:?}",r.kind)};assert_eq!(cards,&vec![f]);answer(&mut g,r,Answer::Cards(vec![f]));done(&mut g);
}
#[test]
fn the_total_stays_locked_when_mana_activation_sacrifices_its_provider() {
    let mut g=game();let tax=put(&mut g,&MANA_TAX,A);let a=put(&mut g,&BEAR,A);let l=put(&mut g,&LAND,A);
    attack(&mut g,&[a]);let r=next(&mut g);let RequestKind::PayCosts{ref options,mana_abilities:true,..}=r.kind else{panic!("{:?}",r.kind)};
    let action=options.iter().find(|o|o.kind==ActionKind::ActivateAbility&&o.source==Some(tax)).unwrap().clone();answer(&mut g,r,Answer::Act(Action::of(&action)));
    choose(&mut g,&[l]);done(&mut g);assert!(g.state.obj(tax).became.is_some());assert!(g.state.obj(l).became.is_some());assert_eq!(g.state.attackers.len(),1);
}
#[test]
fn a_land_creature_sacrificed_to_its_attack_cost_never_becomes_an_attacker() {
    let mut g=game();put(&mut g,&TAX,B);let a=put(&mut g,&LAND_CREATURE,A);attack(&mut g,&[a]);choose(&mut g,&[a]);done(&mut g);assert!(g.state.attackers.is_empty());
}

#[test]
fn a_required_attacker_is_not_forced_to_pay_a_nonmana_cost() {
    let mut g=game();put(&mut g,&TAX,B);let a=put(&mut g,&BEAR,A);
    cx(&mut g).add_effect(a,a,EffectKind::Goaded,Duration::EndOfTurn);
    let r=next(&mut g);let RequestKind::DeclareAttackers{requirements,..}=r.kind else{panic!("{:?}",r.kind)};
    assert_eq!(requirements.most,0,"the payer can always decline the cost to attack");
}
#[test]
fn enlist_can_tap_a_land_creature_before_that_creature_pays_a_sacrifice_cost() {
    let mut g=game();put(&mut g,&TAX,B);let a=put(&mut g,&RECRUITER,A);let l=put(&mut g,&LAND_CREATURE,A);
    attack(&mut g,&[a]);let r=next(&mut g);assert!(matches!(r.kind,RequestKind::ChooseCards{purpose:Purpose::Enlist,..}));answer(&mut g,r,Answer::Cards(vec![l]));
    let r=next(&mut g);let RequestKind::ChooseCostPayment{ref choices,..}=r.kind else{panic!("{:?}",r.kind)};
    let tap=choices.iter().position(|c|c.kind==CostPaymentKind::Tap).unwrap();answer(&mut g,r,Answer::CostPayment(tap));
    choose(&mut g,&[l]);done(&mut g);assert_eq!(g.state.attackers.len(),1);
    let tapped=g.events().iter().position(|r|matches!(r.event,Event::Tapped{permanent} if permanent==l)).unwrap();
    let sacrificed=g.events().iter().position(|r|matches!(r.event,Event::ZoneChange{old,reason:ZoneReason::Sacrifice,..} if old==l)).unwrap();
    assert!(tapped<sacrificed);
}

#[test]
fn sacrificing_an_enlist_payment_first_cannot_later_tap_its_old_identity() {
    let mut g=game();put(&mut g,&TAX,B);let a=put(&mut g,&RECRUITER,A);let l=put(&mut g,&LAND_CREATURE,A);
    attack(&mut g,&[a]);let r=next(&mut g);answer(&mut g,r,Answer::Cards(vec![l]));
    let r=next(&mut g);let RequestKind::ChooseCostPayment{ref choices,..}=r.kind else{panic!("{:?}",r.kind)};
    let sacrifice=choices.iter().position(|c|c.kind==CostPaymentKind::Sacrifice).unwrap();answer(&mut g,r,Answer::CostPayment(sacrifice));
    choose(&mut g,&[l]);let r=next(&mut g);assert!(matches!(r.kind,RequestKind::DeclareAttackers{..}));
    assert!(g.state.obj(l).became.is_none());assert!(!g.state.obj(l).tapped);assert!(!g.state.obj(a).tapped);
    assert!(!g.events().iter().any(|r|matches!(r.event,Event::CombatPaymentBegan{..}|Event::CombatCostObjectPaid{..})));
}
