use cantrip_api::*;
use cantrip_mechanics::attach::{attached_object, equip};
use cantrip_mechanics::statics::has_delayed_ability;
use cantrip_mechanics::{explore, triggers};
use cantrip_oracle as oracle;

/// "Whenever this creature attacks, it explores." (CR 701.44a, 603.2): the equipped creature, which has the ability, is its
/// source, so it is the one that explores and that gets the +1/+1 counter, and its controller who reveals the card.
fn explores(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    explore(g, item.object)
}

/// "Equipped creature has 'You may look at the top card of your library any time' and 'Whenever this creature attacks, it
/// explores.'" (CR 113.6, 613.1f): two abilities the Lantern lists in `delayed`, a play rule and a triggered ability, each
/// given to the equipped creature by its own static ability. Only the creature's controller looks at their library.
pub static CARD: CardDef = CardDef {
    abilities: &[equip(mana("{2}"), "Equip {2}")],
    delayed: &[
        granted_play_rule(PlayRule::LookAtLibraryTop, "You may look at the top card of your library any time."),
        triggered(triggers::THIS_ATTACKS, explores, "Whenever this creature attacks, it explores."),
    ],
    statics: &[
        static_ability(Layer::Ability, attached_object, has_delayed_ability::<0>, "Equipped creature has \"You may look at the top card of your library any time\""),
        static_ability(Layer::Ability, attached_object, has_delayed_ability::<1>, "Equipped creature has \"Whenever this creature attacks, it explores.\""),
    ],
    ..oracle::GLOWCAP_LANTERN
};
