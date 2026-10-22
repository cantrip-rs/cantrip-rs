use cantrip_api::*;
use cantrip_oracle as oracle;

/// The 2/2 red Human Knight creature token with trample and haste.
pub static HUMAN_KNIGHT: CardDef = CardDef {
    name: "Human Knight Token",
    token_name: Some(&[Subtype::Human, Subtype::Knight]),
    has_cost: false,
    colors: Some(ColorSet::of(Color::Red)),
    types: TypeSet::of(&[CardType::Creature]),
    subtypes: SubtypeSet::of(&[Subtype::Human, Subtype::Knight]),
    power: Some(2),
    toughness: Some(2),
    keywords: Keywords::of(&[Keyword::Trample, Keyword::Haste]),
    text: "Trample, haste",
    ..CardDef::BLANK
};

/// Create X Human Knights as one instruction (CR 111.1, 101.4), then the delayed ability that lasts the turn
/// (CR 603.7b). "You" is the spell's controller as it resolved (CR 603.7d).
fn effect(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    cantrip_mechanics::tokens::create(g, item.controller, &HUMAN_KNIGHT, item.x as u32)?;
    cantrip_mechanics::delayed::this_turn(g, item, item.object);
    Ok(())
}

/// "Whenever one or more creatures you control deal combat damage to one or more players this turn" (CR 603.2c): any
/// combat damage above zero to a player from a creature of the ability's controller (the spell's, CR 603.7d); one trigger
/// for the whole batch.
fn creature_deals_combat_damage_to_player(q: &dyn Query, me: &Asker, event: &Event) -> Option<Fired> {
    match event {
        Event::Damage { source: from, target: Target::Player(_), amount, combat: true, .. }
            if *amount > 0 && q.controller(*from) == me.controller && q.chars(*from).is(CardType::Creature) =>
        {
            Fired::PLAIN
        }
        _ => None,
    }
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[], effect),
    delayed: &[triggered(when_asked(EventMask::DAMAGE, creature_deals_combat_damage_to_player), cantrip_mechanics::effects::become_monarch, "Whenever one or more creatures you control deal combat damage to one or more players this turn, you become the monarch.")
        .once_per_batch()],
    ..oracle::FORTH_EORLINGAS
};

pub static EXTRA: &[&CardDef] = &[&HUMAN_KNIGHT];
