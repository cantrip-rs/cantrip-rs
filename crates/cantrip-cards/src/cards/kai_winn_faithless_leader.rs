use cantrip_api::*;
use cantrip_oracle as oracle;

use cantrip_mechanics::library::object_has_name;

/// "As Kai Winn enters, choose a nonland card name." (CR 614.12a, 201.3): recorded as the entering object's chosen name.
fn choose_name(g: &mut dyn Game, entering: &mut Entering) -> Result<(), Suspend> {
    let Answer::Name(name) = g.ask(entering.controller, RequestKind::ChooseName {
        source: entering.object, looked_at: Vec::new(), suggestion: None, kind: NameKind::NONLAND,
    })? else { return Ok(()) };
    entering.chosen_card = name;
    Ok(())
}

fn chosen_spell_or_ability(q: &dyn Query, me: &Asker, event: &Event) -> Option<Fired> {
    let (object, player) = match event {
        Event::SpellCast { spell, player, .. } => (*spell, *player),
        Event::AbilityActivated { source, player, .. } => (*source, *player),
        _ => return None,
    };
    let chosen = q.state().chosen_card(q.linked_to(me))?;
    if q.state().opponents(me.controller).any(|p| p == player) && object_has_name(q, object, chosen) {
        Fired::about(Target::Player(player))
    } else { None }
}

fn lose_life(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if let Target::Player(player) = item.subject { g.lose_life(player, 3); }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    enters: Some(choose_name),
    abilities: &[triggered(
        when_asked(EventMask(EventMask::SPELL_CAST.0 | EventMask::ABILITY_ACTIVATED.0 | EventMask::ZONE_CHANGE.0), chosen_spell_or_ability),
        lose_life,
        "Whenever an opponent casts a spell with the chosen name or activates an ability of a source with the chosen name, that player loses 3 life.",
    )],
    ..oracle::KAI_WINN_FAITHLESS_LEADER
};
