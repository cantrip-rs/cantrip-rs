use cantrip_api::*;
use cantrip_mechanics::{choose, select, targets};
use cantrip_oracle as oracle;

/// The Aura must currently be attached to a creature or land (CR 115.1a).
fn aura_on_creature_or_land(q: &dyn Query, asker: &Asker, t: Target) -> bool {
    matches!(t, Target::Object(id) if targets::permanent(q, id)
        && q.chars(id).has_subtype(q.words(asker.source).subtype(Subtype::Aura))
        && q.state().obj(id).attached_to.is_some_and(|host| {
            targets::permanent(q, host)
                && (q.chars(host).is(CardType::Creature) || q.chars(host).is(CardType::Land))
        }))
}

/// The destination is another permanent of the relevant type which the Aura can enchant (CR 701.3a).
fn destination(q: &dyn Query, choice: &Choice, _: u32) -> Verdict {
    let item = q.state().stack.iter().rev().find(|item| item.object == choice.source)
        .or_else(|| q.state().left_stack(choice.source));
    let aura = item.and_then(|item| q.state().target(item, 0).object());
    Verdict::each(choice, |host| {
        aura.is_some_and(|aura| {
            is_live(q, aura) && q.state().obj(aura).attached_to.is_some_and(|old| {
                host != old && [CardType::Creature, CardType::Land].into_iter()
                    .any(|ty| q.chars(old).is(ty) && q.chars(host).is(ty))
                    && q.can_attach_to(aura, host)
            })
        })
    })
}

fn alter(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(aura) = g.state().target(item, 0).object() else { return Ok(()) };
    let candidates = g.state().battlefield().to_vec();
    let rule = select::Selection::new(select::as_many_as_can(1))
        .with(select::part(&(destination as SelectFn), 0, ""))
        .called("permanent");
    let chosen = choose::select(g, item.controller, item.object, Purpose::Choose, candidates, rule)?;
    if let Some(&host) = chosen.first() {
        instruction(g, |g| { g.attach(aura, host); });
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[target(aura_on_creature_or_land)], alter),
    ..oracle::ENCHANTMENT_ALTERATION
};
