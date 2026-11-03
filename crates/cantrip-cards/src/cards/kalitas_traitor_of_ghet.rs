use cantrip_api::*;
use cantrip_mechanics::{effects, tokens};
use cantrip_oracle as oracle;

/// "If a nontoken creature an opponent controls would die, instead exile that card and create a 2/2 black Zombie creature token." (CR 604.1).
fn if_nontoken_creature_opponent_controls_applies(
    q: &dyn Query,
    asker: &Asker,
    proposal: &Proposal,
) -> bool {
    let Proposal::ZoneChange(z) = proposal else { return false };
    let id = z.object;
    if z.to.is(ZoneKind::Graveyard) && z.from.is(ZoneKind::Battlefield)
        && (!(q.state().obj(id).token) && q.chars(id).is(CardType::Creature)
            && q
                .state()
                .opponents(asker.controller)
                .collect::<Vec<PlayerId>>()
                .contains(&q.controller(id)))
    {
        return true;
    }
    false
}

/// "If a nontoken creature an opponent controls would die, instead exile that card and create a 2/2 black Zombie creature token." (CR 604.1).
fn if_nontoken_creature_opponent_controls_apply(
    q: &dyn Query,
    _: &Asker,
    proposal: &mut Proposal,
) {
    let seen = *proposal;
    let Proposal::ZoneChange(_) = &seen else { return };
    let Proposal::ZoneChange(z) = proposal else { return };
    z.to = Zone::exile(q.state().obj(z.object).owner);
}

/// "If a nontoken creature an opponent controls would die, instead exile that card and create a 2/2 black Zombie creature token." (CR 608.2, 111.1).
fn if_nontoken_creature_opponent_controls_then_steps(
    g: &mut dyn Game,
    asker: &Asker,
    _: Target,
    _: [u32; 2],
) -> Result<(), Suspend> {
    let mut tokens_0 = Vec::new();
    let def_0_0 = g.card_id(&tokens::ZOMBIE);
    tokens_0.extend(std::iter::repeat((def_0_0, asker.controller)).take(1));
    let words_0 = g.words(asker.source);
    g.create_tokens_worded(&tokens_0, false, None, words_0)?;
    Ok(())
}

/// "If a nontoken creature an opponent controls would die, instead exile that card and create a 2/2 black Zombie creature token." (CR 608.2).
fn if_nontoken_creature_opponent_controls_then(
    g: &mut dyn Game,
    asker: &Asker,
    before: &Proposal,
    _: &Proposal,
) {
    let Proposal::ZoneChange(z) = before else { return };
    let id = z.object;
    let subject = Target::Object(id);
    let data = [u32::MAX; 2];
    let _ = if_nontoken_creature_opponent_controls_then_steps(g, asker, subject, data);
}

/// "{2}{B}, Sacrifice another Vampire or Zombie: Put two +1/+1 counters on Kalitas." (CR 608.2).
fn put_two_1_1_counters(q: &dyn Query, asker: &Asker, id: ObjectId) -> bool {
    !(asker.source == id)
        && (q.chars(id).has_subtype(q.words(asker.source).subtype(Subtype::Vampire))
            || q.chars(id).has_subtype(q.words(asker.source).subtype(Subtype::Zombie)))
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        ability(
            cost("{2}{B}").and(sacrifice(1, put_two_1_1_counters)),
            effects::two_plus_one_counters_on_this,
            "{2}{B}, Sacrifice another Vampire or Zombie: Put two +1/+1 counters on Kalitas.",
        ),
    ],
    replacements: &[
        replacement(
                ReplacementKind::ZoneChange,
                if_nontoken_creature_opponent_controls_applies,
                if_nontoken_creature_opponent_controls_apply,
                "If a nontoken creature an opponent controls would die, instead exile that card and create a 2/2 black Zombie creature token.",
            )
            .then(if_nontoken_creature_opponent_controls_then),
    ],
    ..oracle::KALITAS_TRAITOR_OF_GHET
};

pub static EXTRA: &[&CardDef] = &[&tokens::ZOMBIE];
