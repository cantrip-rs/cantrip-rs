use cantrip_api::*;
use cantrip_mechanics::{
    attach::{attached_object, equip},
    choose, effects, select,
    statics::{have_double_strike, plus_pt},
    targets,
    triggers::BEGINNING_OF_YOUR_COMBAT,
};
use cantrip_oracle as oracle;

/// "At the end of combat on your turn" (CR 511.2, 603.2): the end of combat step began during its controller's turn.
fn end_of_combat_on_your_turn(q: &dyn Query, me: &Asker, event: &Event) -> Option<Fired> {
    match event {
        Event::StepBegan { step: Step::EndOfCombat } if q.state().turn.active == me.controller => Fired::PLAIN,
        _ => None,
    }
}

const END_OF_COMBAT_ON_YOUR_TURN: Trigger = when_asked(EventMask::STEP_BEGAN, end_of_combat_on_your_turn);

/// "If a player was dealt 6 or more combat damage this turn" (CR 603.4, 120.2): one player's combat damage this turn added up,
/// from every source and in every combat phase, as the history holds it; damage that was prevented was not dealt. Asked as the
/// ability triggers and again as it resolves.
fn a_player_was_dealt_six_or_more_combat_damage(q: &dyn Query, me: &Asker) -> bool {
    let dealt = |player: PlayerId| -> u32 {
        q.history()
            .damage_dealt()
            .filter(|d| d.combat && d.target == Target::Player(player))
            .map(|d| u32::from(d.amount))
            .sum()
    };
    std::iter::once(me.controller).chain(q.state().opponents(me.controller)).any(|player| dealt(player) >= 6)
}

/// "Transform this enchantment, then attach it to a creature you control" (CR 701.27, 701.3a): it becomes the Equipment, and
/// its controller chooses which creature they control it is attached to as this resolves. Nothing is attached when they
/// control no creature, or when the permanent has left the battlefield (CR 400.7).
fn transform_then_attach(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    g.transform_by(item.object, item)?;
    if !g.is_on_battlefield(item.object) {
        return Ok(());
    }
    let creatures: Vec<ObjectId> = g.permanents_of(item.controller).into_iter().filter(|&id| g.chars(id).is(CardType::Creature)).collect();
    for creature in choose::select(g, item.controller, item.object, Purpose::Attach, creatures, select::creature_permanents(select::exactly(1)))? {
        g.attach(item.object, creature);
    }
    Ok(())
}

pub static BACK: CardDef = CardDef {
    abilities: &[equip(mana("{3}"), "Equip {3}")],
    statics: &[
        static_ability(Layer::PtModify, attached_object, plus_pt::<2, 0>, "Equipped creature gets +2/+0."),
        static_ability(Layer::Ability, attached_object, have_double_strike, "Equipped creature has double strike."),
    ],
    ..oracle::WORLD_CHAMPION_CELESTIAL_WEAPON
};

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            BEGINNING_OF_YOUR_COMBAT,
            effects::target_creature_gets::<2, 0>,
            "At the beginning of combat on your turn, target creature you control gets +2/+0 until end of turn.",
        )
        .targets(&[targets::CREATURE_YOU_CONTROL]),
        triggered(
            END_OF_COMBAT_ON_YOUR_TURN,
            transform_then_attach,
            "At the end of combat on your turn, if a player was dealt 6 or more combat damage this turn, transform this enchantment, then attach it to a creature you control.",
        )
        .condition(a_player_was_dealt_six_or_more_combat_damage),
    ],
    back: Some(&BACK),
    ..oracle::SIDEQUEST_PLAY_BLITZBALL
};
