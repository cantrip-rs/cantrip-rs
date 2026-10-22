use cantrip_api::*;
use cantrip_mechanics::amass::amass;
use cantrip_mechanics::triggers::YOUR_UPKEEP;
use cantrip_oracle as oracle;

/// The 0/0 black Zombie Army creature token amass Zombies makes when its controller has no Army (CR 701.47a).
pub static ZOMBIE_ARMY: CardDef = CardDef {
    name: "Zombie Army Token",
    token_name: Some(&[Subtype::Zombie, Subtype::Army]),
    has_cost: false,
    colors: Some(ColorSet::of(Color::Black)),
    types: TypeSet::of(&[CardType::Creature]),
    subtypes: SubtypeSet::of(&[Subtype::Zombie, Subtype::Army]),
    power: Some(0),
    toughness: Some(0),
    ..CardDef::BLANK
};

/// "At the beginning of your upkeep, you lose 1 life and amass Zombies 1."
fn lose_and_amass(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    g.lose_life(item.controller, 1);
    amass(g, item.controller, item.object, &ZOMBIE_ARMY, Subtype::Zombie, 1)
}

/// "Whenever a Zombie token you control with power 6 or greater attacks" (CR 603.4): a token its controller has, a Zombie,
/// with power 6 or more as the trigger checks - the power is read then, so a change later in the turn does not undo or
/// add the lifelink (rulings 2019-05-03). The ability is about that attacker.
fn zombie_token_attacks(q: &dyn Query, me: &Asker, event: &Event) -> Option<Fired> {
    let Event::Attacks { attacker, .. } = event else { return None };
    let attacker = *attacker;
    let c = q.chars(attacker);
    let zombie = q.obj(attacker).token
        && q.controller(attacker) == me.controller
        && c.has_subtype(q.words(me.source).subtype(Subtype::Zombie))
        && c.power.is_some_and(|power| power >= 6);
    if zombie { Fired::about(Target::Object(attacker)) } else { None }
}

/// "it gains lifelink until end of turn."
fn gain_lifelink(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if let Target::Object(attacker) = item.subject {
        g.add_effect(
            item.object,
            attacker,
            EffectKind::AddKeywords(Keywords::of(&[Keyword::Lifelink])),
            Duration::EndOfTurn,
        );
    }
    Ok(())
}

pub static EXTRA: &[&CardDef] = &[&ZOMBIE_ARMY];

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(YOUR_UPKEEP, lose_and_amass, "At the beginning of your upkeep, you lose 1 life and amass Zombies 1. (Put a +1/+1 counter on an Army you control. It's also a Zombie. If you don't control an Army, create a 0/0 black Zombie Army creature token first.)"),
        triggered(when_asked(EventMask::ATTACKS, zombie_token_attacks), gain_lifelink, "Whenever a Zombie token you control with power 6 or greater attacks, it gains lifelink until end of turn."),
    ],
    ..oracle::DREADHORDE_INVASION
};
