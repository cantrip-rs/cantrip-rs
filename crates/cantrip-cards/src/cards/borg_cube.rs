use cantrip_api::*;
use cantrip_mechanics::{affinity, assimilate, choose, select, station, statics, targets, triggers};
use cantrip_oracle as oracle;

const STATION_TEXT: &str = "Station (Tap another creature you control: Put charge counters equal to its power on this Spacecraft. Station only as a sorcery. It's an artifact creature at 12+.)";
const TWELVE_PLUS: &str = "12+ | Artifact creatures you control have flying.";

/// "Whenever this Spacecraft enters or attacks, assimilate a creature card exiled with it. If you can't, exile up to one
/// target creature." The trigger's own exile instruction and reference form the linked pair (CR 607.2a); it assimilates a
/// linked creature card if possible, otherwise exiles its legal target and links it for the next trigger. A lone available
/// card is forced; otherwise the controller chooses as the ability resolves (CR 608.2d).
fn assimilate_or_exile(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let exiled = g.exiled_with(g.linked(item));
    let eligible: Vec<ObjectId> = exiled.into_iter().filter(|&id| g.chars(id).is(CardType::Creature)).collect();
    let chosen = if eligible.is_empty() {
        None
    } else {
        choose::select(
            g,
            item.controller,
            item.object,
            Purpose::PutOntoBattlefield,
            eligible,
            select::creature_cards(select::exactly(1)),
        )?
        .into_iter()
        .next()
    };
    let assimilated = if let Some(card) = chosen { assimilate::assimilate(g, item, card)?.is_some() } else { false };
    if !assimilated {
        if let Some(creature) = targets::object(g, item, 0) {
            g.exile_with(&[creature], g.linked(item), false);
        }
    }
    Ok(())
}

/// The striation's flying grant applies to artifact creatures its controller controls while this has twelve charge counters.
fn twelve_plus_artifact_creatures(q: &dyn LayerQuery, source: ObjectId, affected: ObjectId, so_far: &Chars) -> bool {
    station::charge(q, source) >= 12 && statics::your_artifact_creatures(q, source, affected, so_far)
}

pub static CARD: CardDef = CardDef {
    own_cost_changes: &[affinity::for_artifact_creatures],
    abilities: &[
        triggered(triggers::THIS_ENTERS_OR_ATTACKS, assimilate_or_exile, "Whenever this Spacecraft enters or attacks, assimilate a creature card exiled with it. If you can't, exile up to one target creature.")
            .targets(&[targets::CREATURE.optional()]),
        station::ability(STATION_TEXT),
    ],
    statics: &[
        static_ability(Layer::Type, station::charged::<12>, station::is_a_creature, "As long as this Spacecraft has twelve or more charge counters on it, it's an artifact creature."),
        // The base-P/T part continues the self-only Type-layer part; keep them adjacent so the flying grant's wider set is separate (CR 613.6, 721.2b).
        static_ability(Layer::PtSet, statics::itself, statics::base_pt::<6, 6>, "12+ | 6/6").continues(),
        static_ability(Layer::Ability, twelve_plus_artifact_creatures, statics::have_flying, TWELVE_PLUS),
    ],
    ..oracle::BORG_CUBE
};
