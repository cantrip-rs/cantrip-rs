//! Oracle text is a verification oracle, never the runtime classifier. Keyword factories must supply their typed tag.
use cantrip_api::*;
fn family(line:&str)->Option<KeywordFamily> {
    if line.trim_start().starts_with("Cumulative upkeep") {Some(KeywordFamily::CumulativeUpkeep)}
    else if line.trim_start().starts_with("Rampage ") {Some(KeywordFamily::Rampage)} else {None}
}
#[test]
fn printed_triggered_keyword_families_are_tagged() {
    let table=CardTable::new(cantrip_cards::all());let mut wrong=Vec::new();let mut checked=0;
    for card in &table.cards {
        for kind in [KeywordFamily::Rampage,KeywordFamily::CumulativeUpkeep] {
            let expected=card.text.lines().filter(|l|family(l)==Some(kind)).count();
            let got=card.abilities.iter().filter(|a|matches!(a,AbilityDef::Triggered(t) if t.keyword_family==Some(kind))).count();
            checked+=expected;if expected!=got {wrong.push(format!("{}: {kind:?} expected {expected}, tagged {got}",card.name));}
        }
    }
    assert!(checked>0);assert!(wrong.is_empty(),"{}",wrong.join("\n"));
}
