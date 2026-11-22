use cantrip_api::*;
use cantrip_mechanics::modified::is_modified_in_layers;
use cantrip_mechanics::statics::have_flying;
use cantrip_mechanics::triggers;
use cantrip_oracle as oracle;

/// "This creature ... as long as it's modified" (CR 700.9, 604.1, 613.1f): the source while a counter, an Equipment or an Aura its
/// controller controls is on it, re-asked on every pass, so it has flying from the moment it is modified and loses it with the last
/// modification.
fn itself_while_modified(q: &dyn LayerQuery, source: ObjectId, affected: ObjectId, _: &Chars) -> bool {
    source == affected && is_modified_in_layers(q, source)
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggers::ward::<2>("Ward {2} (Whenever this creature becomes the target of a spell or ability an opponent controls, counter it unless that player pays {2}.)")],
    statics: &[static_ability(
        Layer::Ability,
        itself_while_modified,
        have_flying,
        "This creature has flying as long as it's modified. (Equipment, Auras you control, and counters are modifications.)",
    )],
    ..oracle::SKYWARD_SPIDER
};
