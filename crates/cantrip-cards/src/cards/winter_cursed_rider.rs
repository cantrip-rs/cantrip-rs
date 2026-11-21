use cantrip_api::*;
use cantrip_mechanics::{effects, statics, targets::filters, triggers::ward_life};
use cantrip_oracle as oracle;

/// "Ward—Pay 2 life." (CR 702.21a), Winter's own and, through the static, the one each artifact its controller has gets (CR 613.1f,
/// layer 6: `grant_delayed_ability` gives the ward in `delayed` to every artifact you control, judged as the earlier layers made it).
const WARD: AbilityDef = ward_life::<2>("Ward\u{2014}Pay 2 life. (Whenever this permanent becomes the target of a spell or ability an opponent controls, counter it unless that player pays 2 life.)");

/// "Each other nonartifact creature gets -X/-X until end of turn": every creature on the battlefield that is not an artifact and
/// not Winter, as this resolves and fixed then (CR 611.2c), for the X announced when the cost was paid (CR 107.3a, 602.2b) -
/// the number of artifact cards exiled from the graveyard, all in one instruction. An X of 0 changes nothing.
fn shrink_the_others(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let x = item.x as i32;
    let winter = item.object;
    effects::pump_creatures_where(g, winter, |q, id| id != winter && !q.chars(id).is(CardType::Artifact), -x, -x);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        WARD,
        ability(
            cost("{2}{U}{B}, {T}").and(measured(CostTake::ExileFromGraveyard, CostMeasure::Count, filters::artifact)),
            shrink_the_others,
            "Exhaust \u{2014} {2}{U}{B}, {T}, Exile X artifact cards from your graveyard: Each other nonartifact creature gets -X/-X until end of turn. (Activate each exhaust ability only once.)",
        )
        .exhaust(),
    ],
    delayed: &[WARD],
    statics: &[static_ability(Layer::Ability, statics::your_artifacts, statics::grant_delayed_ability, "Artifacts you control have \"Ward\u{2014}Pay 2 life.\"")],
    ..oracle::WINTER_CURSED_RIDER
};
