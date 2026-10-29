use cantrip_api::*;
use cantrip_mechanics::tokens::{self, NINJA};
use cantrip_mechanics::triggers::DEALS_COMBAT_DAMAGE_TO_PLAYER_SUBJECT;
use cantrip_mechanics::{effects, lands, sneak, statics};
use cantrip_oracle as oracle;

/// "Attacking Ninjas you control" (CR 506.4, 613.1f): an attacking creature of yours that is a Ninja, judged on what the earlier
/// layers made it.
fn attacking_ninjas_you_control(view: &dyn LayerQuery, source: ObjectId, affected: ObjectId, so_far: &Chars) -> bool {
    statics::attacking_creatures_you_control(view, source, affected, so_far)
        && statics::your_of_subtype::<{ lands::sub(Subtype::Ninja) }>(view, source, affected, so_far)
}

/// "Create a 1/1 black Ninja creature token. Then if you control five or more Ninjas, that player loses half their life, rounded
/// up" (CR 107.1a, 119.3): the Ninjas counted once the token is there, this Ninja and the token among them, and the damaged
/// player the trigger's subject.
fn make_a_ninja_then_drain_half(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    tokens::create(g, item.controller, &NINJA, 1)?;
    let ninja = g.words(item.object).subtype(Subtype::Ninja);
    let ninjas = g
        .state()
        .battlefield()
        .iter()
        .filter(|&&id| g.obj(id).controller == item.controller && g.chars(id).has_subtype(ninja))
        .count();
    if ninjas >= 5
        && let Target::Player(player) = item.subject
    {
        effects::lose_half_their_life(g, player);
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    statics: &[static_ability(Layer::Ability, attacking_ninjas_you_control, statics::have_deathtouch, "Attacking Ninjas you control have deathtouch.")],
    abilities: &[triggered(
        DEALS_COMBAT_DAMAGE_TO_PLAYER_SUBJECT,
        make_a_ninja_then_drain_half,
        "Whenever Dark Leo & Shredder deal combat damage to a player, create a 1/1 black Ninja creature token. Then if you control five or more Ninjas, that player loses half their life, rounded up.",
    )],
    ways: &[sneak::way(mana("{W}{B}"), "Sneak {W}{B}")],
    ..oracle::DARK_LEO_SHREDDER
};

/// The Ninja token it makes.
pub static EXTRA: &[&CardDef] = &[&NINJA];
