use cantrip_api::*;
use cantrip_mechanics::effects;
use cantrip_mechanics::triggers::BEGINNING_OF_YOUR_COMBAT;
use cantrip_oracle as oracle;

const ASK: Question = Question::new(
    "JohanCantAttack",
    "have Johan gain \"Johan can't attack\" until end of combat?",
    "If you do, attacking doesn't cause creatures you control to tap this combat while Johan is untapped",
    "Johan can't attack",
    "Don't",
);

/// "At the beginning of combat on your turn, you may have Johan gain "Johan can't attack" until end of combat. If you do,
/// attacking doesn't cause creatures you control to tap this combat if Johan is untapped." Johan gains an ability (CR 113.1a,
/// 613.1f), which losing its abilities takes; the second sentence is a rule of the game about an open set (CR 611.2c), no
/// ability of Johan's or of the creatures, so it reads whichever creatures you control as attackers are declared (CR
/// 508.1f). Johan gone by then is a new object (CR 400.7) that can't gain anything: nothing is asked, and nothing happens.
fn hold_back(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if !effects::ask_may_this_card(g, item, Purpose::Ask(&ASK))? {
        return Ok(());
    }
    g.add_effect(item.object, item.object, EffectKind::AddKeywords(Keywords::of(&[Keyword::CantAttack])), Duration::EndOfCombat);
    g.end_sentence();
    g.add_lasting_rule(item, 0, Target::None, Duration::EndOfCombat);
    Ok(())
}

/// "Attacking doesn't cause creatures you control to tap this combat if Johan is untapped" (CR 508.1f): a creature its
/// controller ("you", the ability's controller) controls, while Johan is still the untapped permanent it was - a Johan
/// that has left the battlefield is neither (CR 400.7, 110.5).
fn your_creatures_while_johan_untapped(q: &dyn Query, asker: &Asker, _: Target, id: ObjectId) -> bool {
    let johan = q.obj(asker.source);
    is_live(q, asker.source)
        && johan.zone.is(ZoneKind::Battlefield)
        && !johan.tapped
        && q.controller(id) == asker.controller
        && q.chars(id).is(CardType::Creature)
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        BEGINNING_OF_YOUR_COMBAT,
        hold_back,
        "At the beginning of combat on your turn, you may have Johan gain \"Johan can't attack\" until end of combat. If you do, attacking doesn't cause creatures you control to tap this combat if Johan is untapped.",
    )],
    lasting_rules: &[LastingRule {
        text: "Attacking doesn't cause creatures you control to tap this combat if Johan is untapped.",
        rule: LastingRuleKind::AttackingDoesntCauseTap,
        applies: your_creatures_while_johan_untapped,
    }],
    ..oracle::JOHAN
};
