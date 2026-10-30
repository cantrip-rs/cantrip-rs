use cantrip_api::*;
use cantrip_mechanics::statics;
use cantrip_oracle as oracle;

/// "As long as another creature entered the battlefield under your control this turn" (CR 611.3a): asked of the history
/// (`Event::Entered`, each creature as it entered, under the controller it entered under, CR 603.6a) whenever the layers apply,
/// so it holds from the moment that creature enters until the turn ends, whether it has since left, been stolen or stopped being
/// a creature, or not.
fn itself_after_another_creature_entered(view: &dyn LayerQuery, source: ObjectId, affected: ObjectId, so_far: &Chars) -> bool {
    statics::itself(view, source, affected, so_far)
        && view.history().entered().any(|e| e.permanent != source && e.controller == view.state().obj(source).controller && e.is(CardType::Creature))
}

fn plus_two_power(_: &dyn LayerQuery, _: ObjectId, chars: &mut Chars) {
    chars.power = chars.power.map(|v| v + 2);
}

pub static CARD: CardDef = CardDef {
    // "This creature can't be blocked by creatures with power 2 or less" (CR 509.1b): checked as blockers are declared, with the
    // power the blocker has then.
    keywords: Keywords::of(&[Keyword::CantBeBlockedByPowerTwoOrLess]),
    statics: &[static_ability(
        Layer::PtModify,
        itself_after_another_creature_entered,
        plus_two_power,
        "As long as another creature entered the battlefield under your control this turn, this creature gets +2/+0.",
    )],
    ..oracle::BRISTLEBANE_OUTRIDER
};
