use cantrip_api::*;
use cantrip_mechanics::effects::plus_one_counter_on_this;
use cantrip_mechanics::{choose, select};
use cantrip_oracle as oracle;

/// "Each player sacrifices half the non-God creatures they control of their choice, rounded down" (CR 701.21a, 101.4, 107.1a): each
/// player in turn order chooses, all on the board as it is before anything is sacrificed (the ruling), the number being half of the
/// non-God creatures they control; then every choice is sacrificed at once. Of those a player may be made to sacrifice (CR 101.2), a
/// player with fewer than the half available sacrifices all of those.
fn each_player_sacrifices_half(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let mut doomed: Vec<(PlayerId, ObjectId)> = Vec::new();
    let god = g.words(item.object).subtype(Subtype::God);
    for who in g.state().apnap().collect::<Vec<_>>() {
        let creatures: Vec<ObjectId> = g.permanents_of(who).into_iter().filter(|&id| g.chars(id).is(CardType::Creature) && !g.chars(id).has_subtype(god)).collect();
        let half = creatures.len() / 2;
        let eligible: Vec<ObjectId> = creatures.into_iter().filter(|&id| g.can_sacrifice(who, id)).collect();
        let count = half.min(eligible.len()) as u32;
        let chosen = choose::select(g, who, item.object, Purpose::Sacrifice, eligible, select::cards(select::exactly(count)))?;
        doomed.extend(chosen.into_iter().map(|id| (who, id)));
    }
    g.sacrifice_each(&doomed);
    Ok(())
}

/// "Whenever a player sacrifices another creature" (CR 701.21a, 603.10a): any player's creature, as it last was on the battlefield.
/// Zodiark sacrificing itself puts the counter on nothing (CR 400.7), so "another" needs no test of its own.
fn a_player_sacrifices_another_creature(q: &dyn Query, _: &Asker, event: &Event) -> Option<Fired> {
    match event {
        Event::ZoneChange { old, from, reason: ZoneReason::Sacrifice, .. } if from.is(ZoneKind::Battlefield) && q.chars(*old).is(CardType::Creature) => Fired::PLAIN,
        _ => None,
    }
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            cantrip_mechanics::triggers::THIS_ENTERS,
            each_player_sacrifices_half,
            "When Zodiark enters, each player sacrifices half the non-God creatures they control of their choice, rounded down.",
        ),
        triggered(
            when_asked(EventMask::ZONE_CHANGE, a_player_sacrifices_another_creature),
            plus_one_counter_on_this,
            "Whenever a player sacrifices another creature, put a +1/+1 counter on Zodiark.",
        )
        .on_sacrifice(),
    ],
    ..oracle::ZODIARK_UMBRAL_GOD
};
