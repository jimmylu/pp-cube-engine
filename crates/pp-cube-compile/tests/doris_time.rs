mod support;
use pp_cube_compile::{CompileContext, DialectId};
use std::sync::Arc;

fn compiled(rest: &str) -> pp_cube_compile::Compiled {
    compiled_in(rest, "Asia/Tokyo", vec![])
}
fn compiled_in(
    rest: &str,
    zone: &str,
    security: Vec<pp_cube_model::Filter>,
) -> pp_cube_compile::Compiled {
    let model = Arc::new(pp_cube_model::load_yaml(include_str!("fixtures/sales.yaml")).unwrap());
    let query =
        pp_cube_model::load_query(&format!(r#"{{"measures":["sales.amount"],{rest}}}"#)).unwrap();
    support::compile(
        &model,
        &query,
        &CompileContext {
            request_id: String::new(),
            dialect: DialectId::Doris,
            timezone: zone.into(),
            security_filters: security,
        },
    )
    .unwrap()
}
// scenario: B0-DORIS-009
#[test]
fn temporal_goldens() {
    for unit in ["day", "week", "month", "quarter", "year"] {
        let result = compiled(&format!(
            r#""timeDimensions":[{{"dimension":"sales.sold_at","granularity":"{unit}","dateRange":["2026-10-01","2026-10-31"]}}]"#
        ));
        insta::assert_snapshot!(
            unit,
            format!("{}\n-- parameters: {:?}", result.sql, result.parameters)
        );
    }
    for op in [
        "inDateRange",
        "notInDateRange",
        "beforeDate",
        "beforeOrOnDate",
        "afterDate",
        "afterOrOnDate",
    ] {
        let values = if op.ends_with("Range") {
            r#"["2026-10-01","2026-10-31"]"#
        } else {
            r#"["2026-10-01"]"#
        };
        let result = compiled(&format!(
            r#""filters":[{{"member":"sales.sold_at","operator":"{op}","values":{values}}}]"#
        ));
        insta::assert_snapshot!(
            op,
            format!("{}\n-- parameters: {:?}", result.sql, result.parameters)
        );
    }
}

// scenario: B0-DORIS-009
#[test]
fn timezone_precedence_and_temporal_metadata() {
    let result = compiled(
        r#""timezone":"UTC","timeDimensions":[{"dimension":"sales.sold_at","granularity":"day"}]"#,
    );
    assert!(result.sql.contains("'UTC', 'UTC'"));
    assert_eq!(result.columns[0].alias, "sales__sold_at_day");
    assert_eq!(result.columns[0].granularity.as_deref(), Some("day"));
    assert_eq!(result.used_cubes.len(), 1);
}
