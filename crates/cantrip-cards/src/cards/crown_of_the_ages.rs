use cantrip_api::*;
use cantrip_mechanics::{choose, select};
use cantrip_oracle as oracle;

/// "{4}, {T}: Attach target Aura attached to a creature to another creature." (CR 115.1).
fn attach_target_aura_attached_creature_target_0(
    q: &dyn Query,
    asker: &Asker,
    t: Target,
) -> bool {
    matches!(
        t, Target::Object(id) if cantrip_mechanics::targets::permanent(q, id) && ((q
        .chars(id).has_subtype(q.words(asker.source).subtype(Subtype::Aura)) && q.state()
        .obj(id).attached_to.is_some_and(| host | q.state().battlefield().iter().copied()
        .filter(| & card_in_zone | q.chars(card_in_zone).is(CardType::Creature))
        .collect:: < Vec < ObjectId > > ().contains(& host))))
    )
}

fn legal_other_host(q: &dyn Query, choice: &cantrip_api::select::Choice, aura: u32) -> cantrip_api::select::Verdict {
    let aura = ObjectId(aura);
    cantrip_api::select::Verdict::each(choice, |host| {
        is_live(q, aura)
            && q.state().obj(aura).attached_to != Some(host)
            && q.chars(host).is(CardType::Creature)
            && q.can_attach_to(aura, host)
    })
}

/// "{4}, {T}: Attach target Aura attached to a creature to another creature." (CR 608.2, 701.3a).
fn attach_target_aura_attached_creature(
    g: &mut dyn Game,
    item: &StackItem,
) -> Result<(), Suspend> {
    let Some(aura) = g.state().target(item, 0).object() else { return Ok(()) };
    let candidates = g.state().battlefield().to_vec();
    let rule = select::Selection::new(select::as_many_as_can(1))
        .with(select::part(&(legal_other_host as cantrip_api::select::SelectFn), aura.0, ""))
        .called("other creature");
    let chosen = choose::select(g, item.controller, item.object, Purpose::Choose, candidates, rule)?;
    if let Some(&host) = chosen.first() {
        instruction(g, |g| { g.attach(aura, host); });
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        ability(
                cost("{4}").and(TAP),
                attach_target_aura_attached_creature,
                "{4}, {T}: Attach target Aura attached to a creature to another creature.",
            )
            .targets(&[target(attach_target_aura_attached_creature_target_0)]),
    ],
    ..oracle::CROWN_OF_THE_AGES
};
