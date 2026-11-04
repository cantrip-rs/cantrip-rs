use cantrip_api::*;
use cantrip_mechanics::{attach, statics, targets};
use cantrip_oracle as oracle;

/// The enchanted creature is blocking and the Aura's controller controls a snow land (CR 604.1).
fn blocking_with_snow_land(q: &dyn LayerQuery, source: ObjectId, affected: ObjectId, chars: &Chars) -> bool {
    attach::attached_object(q, source, affected, chars)
        && q.state().blockers.contains(&affected)
        && q.state().battlefield().iter().any(|&id| {
            let land = q.chars(id);
            q.controller(id) == q.controller(source) && land.supertypes.snow && land.is(CardType::Land)
        })
}

pub static CARD: CardDef = CardDef {
    enchant: Some(targets::CREATURE),
    statics: &[
        static_ability(Layer::Ability, attach::attached_object, statics::have_flying, "Enchanted creature has flying."),
        static_ability(Layer::Ability, blocking_with_snow_land, statics::have_first_strike, "Enchanted creature has first strike as long as it's blocking and you control a snow land."),
    ],
    ..oracle::SNOW_DEVIL
};
