//! Printed return clauses from Puppet Master and Genju of the Fields exercise CR400.7f over real player questions.
//! Test-only definitions; Genju's unrelated animation ability is not used by this fixture.
use cantrip_api::Game as Rules;
use cantrip_engine::*;
use cantrip_mechanics::{effects::ask_may, targets};
use cantrip_script::{GameScript, run_on};
use std::sync::OnceLock;
fn dies(q: &dyn Query, source: ObjectId, event: &Event) -> Option<Fired> {
    let Event::ZoneChange {
        old, new, from, to, ..
    } = *event
    else {
        return None;
    };
    (from.is(ZoneKind::Battlefield)
        && to.is(ZoneKind::Graveyard)
        && q.obj(source).attached_to == Some(old))
    .then_some(Fired {
        subject: Target::Object(old),
        data: [new.0, 0],
    })
}
fn puppet(g: &mut dyn Rules, item: &StackItem) -> Result<(), Suspend> {
    puppet_return(g, item, true)
}
fn puppet_old_source(g: &mut dyn Rules, item: &StackItem) -> Result<(), Suspend> {
    puppet_return(g, item, false)
}
fn puppet_return(g: &mut dyn Rules, item: &StackItem, tracked: bool) -> Result<(), Suspend> {
    let host = item.subject.object().unwrap();
    let dead = g
        .found_all_after_move(host)
        .into_iter()
        .filter(|&id| g.obj(id).zone.is(ZoneKind::Graveyard))
        .collect::<Vec<_>>();
    let returned = g.return_to_hand(&dead);
    g.end_sentence();
    if returned.iter().any(|&id| g.obj(id).zone.is(ZoneKind::Hand))
        && g.may_pay(
            item.controller,
            item.object,
            item.object,
            cost("{U}{U}{U}"),
            &PAY,
        )?
    {
        g.end_sentence();
        let aura = if tracked {
            g.found_aura_after_enchanted_left(item.object, host)
                .into_iter()
                .collect::<Vec<_>>()
        } else {
            vec![item.object]
        };
        g.return_to_hand(&aura);
    }
    Ok(())
}
static PAY: Question = Question::new("Pay", "pay {cost}?", "", "Pay {cost}", "Decline");
static PUPPET: CardDef = CardDef {
    name: "Puppet Master",
    oracle: true,
    cost: mana("{U}{U}{U}"),
    has_cost: true,
    types: TypeSet::of(&[CardType::Enchantment]),
    subtypes: SubtypeSet::of(&[Subtype::Aura]),
    enchant: Some(targets::CREATURE),
    abilities: &[triggered(
        when(EventMask::ZONE_CHANGE, dies),
        puppet,
        "When enchanted creature dies, return that card to its owner's hand. If that card is returned to its owner's hand this way, you may pay {U}{U}{U}. If you do, return this card to its owner's hand.",
    )],
    ..CardDef::BLANK
};
fn plains(q: &dyn Query, _: &Asker, t: Target) -> bool {
    t.object()
        .is_some_and(|id| q.is_on_battlefield(id) && q.chars(id).has_subtype(Subtype::Plains))
}
fn genju(g: &mut dyn Rules, item: &StackItem) -> Result<(), Suspend> {
    let Some(aura) = g.found_aura_after_enchanted_left(item.object, item.subject.object().unwrap())
    else {
        return Ok(());
    };
    if ask_may(g, item.controller, item.object, Purpose::Optional)? {
        g.return_to_hand(&[aura]);
    }
    Ok(())
}
static GENJU: CardDef = CardDef {
    name: "Genju of the Fields",
    oracle: true,
    cost: mana("{W}"),
    has_cost: true,
    types: TypeSet::of(&[CardType::Enchantment]),
    subtypes: SubtypeSet::of(&[Subtype::Aura]),
    enchant: Some(target(plains)),
    abilities: &[triggered(
        when(EventMask::ZONE_CHANGE, dies),
        genju,
        "When enchanted Plains is put into a graveyard, you may return this card from your graveyard to your hand.",
    )],
    ..CardDef::BLANK
};
fn table() -> &'static CardTable {
    static T: OnceLock<CardTable> = OnceLock::new();
    // Production Puppet Master can now be present. These scenarios deliberately exercise the local fixtures.
    let table = T.get_or_init(|| CardTable::new(cantrip_cards::all().into_iter().filter(|c| !matches!(c.name, "Puppet Master" | "Genju of the Fields")).chain([&PUPPET, &GENJU])));
    assert_eq!(table.find("Puppet Master"), Some(table.id_of(&PUPPET)));
    assert_eq!(table.find("Genju of the Fields"), Some(table.id_of(&GENJU)));
    table
}
fn paid_puppet_fixture() -> String {
    include_str!("fixtures/aura_identity/puppet-master-successful-return-and-payment-returns-aura.cantrip").to_owned()
}
fn play(text: &str, name: &str) {
    let script = GameScript::parse(text, name).unwrap();
    if let Err(e) = run_on(&script, table()) {
        panic!("{}", e.report)
    }
}
#[test]
fn retained_puppet_paid_control_returns_aura() {
    play(
        &paid_puppet_fixture(),
        "puppet-master-successful-return-and-payment-returns-aura",
    )
}
#[test]
fn genju_returns_from_unattached_aura_sba() {
    play(
        r#"
[Bot1 Deck]
Plains
Genju of the Fields
Island x25
[Bot2 Deck]
Strip Mine
Island x25
[Actions]
Bot1.Hand.Plains.Play
Bot1.Battlefield.Plains.Activate
Bot1.Hand.Genju of the Fields.Cast(Bot1.Battlefield.Plains)
[T2 Bot2.Main1]
Bot2.Hand.Strip Mine.Play
Bot2.Battlefield.Strip Mine.Activate(Ability: Destroy; Bot1.Battlefield.Plains)
Bot1.Accept
[T2 Bot2.Main2]
[Expect]
Bot1.Graveyard = Plains
Bot1.Hand = Genju of the Fields; Island; Island; Island; Island; Island
"#,
        "genju-unattached-aura-return",
    )
}

#[test]
fn original_source_recipe_reproduces_paid_puppet_runtime_failure() {
    static OLD: CardDef = CardDef {
        abilities: &[triggered(
            when(EventMask::ZONE_CHANGE, dies),
            puppet_old_source,
            "When enchanted creature dies, return that card to its owner's hand. If that card is returned to its owner's hand this way, you may pay {U}{U}{U}. If you do, return this card to its owner's hand.",
        )],
        ..PUPPET
    };
    static T: OnceLock<CardTable> = OnceLock::new();
    // This is an intentionally incorrect source-identity recipe, not a requirement that the fixed card stay broken.
    let table = T.get_or_init(|| CardTable::new(cantrip_cards::all().into_iter().filter(|c| c.name != "Puppet Master").chain([&OLD])));
    assert_eq!(table.find("Puppet Master"), Some(table.id_of(&OLD)));
    let text = paid_puppet_fixture();
    let script = GameScript::parse(&text, "puppet-paid-original-source").unwrap();
    let failure = match run_on(&script, table) {
        Err(failure) => failure,
        Ok(_) => panic!("old battlefield source identity must miss the Aura card"),
    };
    assert!(
        failure
            .report
            .contains("Hand.Count expected \"3\", was \"2\""),
        "{}",
        failure.report
    );
    println!("Original Source control: {}", failure.report);
}
