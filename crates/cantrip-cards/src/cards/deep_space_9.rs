use cantrip_api::*;
use cantrip_mechanics::{station, statics, targets, triggers};
use cantrip_oracle as oracle;

const STATION_TEXT: &str = "Station (Tap another creature you control: Put charge counters equal to its power on this Spacecraft. Station only as a sorcery.)";
const NINE_PLUS: &str = "9+ | Creatures you control get +3/+3.";

/// Exile the top four together, then put the permanent cards that really arrived in exile onto the battlefield under the
/// ability controller's control (CR 406.2, 608.2f). Cards sent elsewhere by a replacement are not among those exiled.
fn exile_and_put_permanents(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let player = item.controller;
    let top = cantrip_mechanics::top_of_library(g, player, 4);
    let exiled: Vec<ObjectId> = g.exile_all(&top).into_iter().filter(|&card| g.obj(card).zone.is(ZoneKind::Exile)).collect();
    let entering: Vec<(ObjectId, PlayerId)> = exiled
        .into_iter()
        .filter(|&card| targets::is_permanent_card(&g.chars(card)))
        .map(|card| (card, player))
        .collect();
    if !entering.is_empty() {
        g.put_all_onto_battlefield(&entering)?;
    }
    Ok(())
}

fn your_creatures_at_nine_charge(q: &dyn LayerQuery, source: ObjectId, affected: ObjectId, chars: &Chars) -> bool {
    station::charge(q, source) >= 9 && statics::your_creatures(q, source, affected, chars)
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            triggers::THIS_ENTERS_OR_YOUR_UPKEEP,
            exile_and_put_permanents,
            "When Deep Space 9 enters and at the beginning of your upkeep, exile the top four cards of your library. Put all permanent cards exiled this way onto the battlefield.",
        ),
        station::ability(STATION_TEXT),
    ],
    statics: &[static_ability(Layer::PtModify, your_creatures_at_nine_charge, statics::plus_pt::<3, 3>, NINE_PLUS)],
    ..oracle::DEEP_SPACE_9
};
