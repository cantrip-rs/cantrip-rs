use cantrip_api::*;
use cantrip_mechanics::{choose_color, effects, targets};
use cantrip_oracle as oracle;

/// The creature The Aetherspark is attached to, if it is still the creature on the battlefield it was attached to.
fn host(view: &dyn LayerQuery, source: ObjectId) -> Option<ObjectId> {
    let host = view.state().obj(source).attached_to?;
    let h = view.state().obj(host);
    (h.became.is_none() && h.zone.is(ZoneKind::Battlefield) && view.chars(host).is(CardType::Creature)).then_some(host)
}

fn while_attached_to_a_creature(view: &dyn LayerQuery, source: ObjectId, affected: ObjectId, _: &Chars) -> bool {
    source == affected && host(view, source).is_some()
}

/// "As long as The Aetherspark is attached to a creature, it can't be attacked (CR 508.1c) and has \"Whenever equipped
/// creature deals combat damage during your turn, put that many loyalty counters on The Aetherspark.\"": one static
/// ability, in layer 6, so losing all abilities takes both away together.
fn grant(view: &dyn LayerQuery, source: ObjectId, chars: &mut Chars) {
    chars.keywords = chars.keywords.with(Keyword::CantBeAttacked);
    let o = view.state().obj(source);
    let card = o.copied_card().unwrap_or(o.card);
    if let Some(slot) = chars.abilities.iter().position(|a| a.is_none()) {
        chars.abilities[slot] = Some(AbilityRef::delayed(card, 0));
    }
}

/// "Whenever equipped creature deals combat damage during your turn": all the combat damage the equipped creature
/// dealt in the one event, to anything, as one trigger (CR 603.2c) carrying that much.
fn equipped_creature_deals_combat_damage(q: &dyn Query, source: ObjectId, event: &Event) -> Option<Fired> {
    match event {
        Event::Damage { source: from, amount, kind, .. }
            if kind.is_combat() && *amount > 0 && q.attached_to(source) == Some(*from) && q.state().turn.active == q.controller(source) =>
        {
            Some(Fired { subject: Target::Object(*from), data: [0, 0] })
        }
        _ => None,
    }
}

fn put_loyalty(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if is_live(g, item.object) && g.is_on_battlefield(item.object) && item.matched_damage_amount() > 0 {
        g.add_counters(item.object, CounterKind::Loyalty, item.matched_damage_amount() as i32);
    }
    Ok(())
}

/// +1: attach to up to one target creature you control, and put a +1/+1 counter on that creature.
fn plus_one(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if let Some(creature) = targets::object(g, item, 0) {
        g.attach(item.object, creature);
        g.add_counters(creature, CounterKind::PlusOne, 1);
    }
    Ok(())
}

/// A loyalty ability, not a mana ability (CR 605.1a): it uses the stack, and the one color is chosen as it resolves.
fn minus_ten(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let color = choose_color(g, item.controller, item.object)?;
    g.add_mana(item.controller, color.mana(), 10);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        ability(loyalty(1), plus_one, "+1: Attach The Aetherspark to up to one target creature you control. Put a +1/+1 counter on that creature.")
            .targets(&[targets::CREATURE_YOU_CONTROL.optional()]),
        ability(loyalty(-5), |g, item| effects::draw(g, item, 2), "\u{2212}5: Draw two cards."),
        ability(loyalty(-10), minus_ten, "\u{2212}10: Add ten mana of any one color."),
    ],
    delayed: &[triggered(when(EventMask::DAMAGE, equipped_creature_deals_combat_damage), put_loyalty, "Whenever equipped creature deals combat damage during your turn, put that many loyalty counters on The Aetherspark.").damage_amount()
        .sums_per_subject()],
    statics: &[static_ability(Layer::Ability, while_attached_to_a_creature, grant, "As long as The Aetherspark is attached to a creature, The Aetherspark can't be attacked and has \"Whenever equipped creature deals combat damage during your turn, put that many loyalty counters on The Aetherspark.\"")],
    ..oracle::THE_AETHERSPARK
};
