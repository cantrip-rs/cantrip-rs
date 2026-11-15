use cantrip_api::*;
use cantrip_mechanics::{choose, select, triggers};
use cantrip_oracle as oracle;

/// "Players can't play lands" (CR 101.2, 305.1): any player, any land, from anywhere and whatever allows the play. A land put
/// onto the battlefield is not played; the next line is about that.
fn players_cant_play_lands(_: &dyn Query, _: &Asker, _: PlayerId, _: ObjectId, _: CardId) -> bool {
    false
}

/// "Lands can't enter the battlefield" (CR 101.2, 614.17): any permanent that would be a land there, judged as it would exist
/// on the battlefield (CR 614.17d) - a Clone entering as a copy of Dryad Arbor can't (the example of CR 608.3e), nor can a
/// token copy of a land. The effect putting it there does the rest of what it says (the rulings).
fn lands_cant_enter(q: &dyn Query, _: &Asker, permanent: ObjectId) -> bool {
    !q.chars(permanent).is(CardType::Land)
}

const SACRIFICE: u8 = 0;
const DAMAGE: u8 = 1;
const NEITHER: u8 = 2;

/// "At the beginning of each upkeep, any player may sacrifice two lands of their choice or have this enchantment deal 5
/// damage to that player. If a player does either, destroy this enchantment." Each player in turn order from the active
/// player chooses one or neither (CR 101.4: Vexing Devil's rulings for "any opponent may"), sacrificing being offered only to
/// one with two lands they can sacrifice (CR 101.2), and choosing which two as they choose it; then everything chosen
/// happens at once (CR 101.4): the damage to each player who chose it (CR 120.3) and every sacrifice (CR 701.21a), one event,
/// so a land sacrificed in it doesn't see the damage (CR 603.10). A player who chose the damage did it though it was
/// prevented (Vexing Devil's ruling), and a player who chose to sacrifice did it once the lands have left; then this
/// enchantment is destroyed (CR 701.8a) - not if it is a new object by then (CR 400.7).
fn any_player_may_sacrifice_two_lands_or_take_five(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let card = g.obj(item.object).card;
    let mut damage: Vec<(ObjectId, Target, u32)> = Vec::new();
    let mut sacrifices: Vec<(PlayerId, ObjectId)> = Vec::new();
    for player in g.state().apnap().collect::<Vec<_>>() {
        let lands: Vec<ObjectId> = g
            .permanents_of(player)
            .into_iter()
            .filter(|&id| g.chars(id).is(CardType::Land) && g.can_sacrifice(player, id))
            .collect();
        let mut options = Vec::new();
        if lands.len() >= 2 {
            options.push(SACRIFICE);
        }
        options.extend([DAMAGE, NEITHER]);
        let labels = options.iter().map(|&i| Label::Card(card, i)).collect();
        match options[choose::one_of(g, player, item.object, labels)? as usize] {
            SACRIFICE => {
                let two = choose::select(g, player, item.object, Purpose::Sacrifice, lands, select::land_cards(select::exactly(2)))?;
                sacrifices.extend(two.into_iter().map(|land| (player, land)));
            }
            DAMAGE => damage.push((item.object, Target::Player(player), 5)),
            _ => {}
        }
    }
    g.together(Together { sacrifices: &sacrifices, damage: &damage, ..Together::default() });
    let sacrificed = sacrifices.iter().any(|&(_, land)| !g.is_on_battlefield(land));
    if !damage.is_empty() || sacrificed {
        g.destroy(item.object);
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        triggers::EACH_UPKEEP,
        any_player_may_sacrifice_two_lands_or_take_five,
        "At the beginning of each upkeep, any player may sacrifice two lands of their choice or have this enchantment deal 5 damage to that player. If a player does either, destroy this enchantment.",
    )],
    labels: &["Sacrifice two lands", "Take 5 damage", "Neither"],
    land_play_rules: &[players_cant_play_lands],
    permanent_enter_rules: &[lands_cant_enter],
    ..oracle::WORMS_OF_THE_EARTH
};
