use {
    crate::{row, select, select_with_null, stringify_label, test_case},
    gluesql_core::{
        error::TranslateError,
        executor::EvaluateError,
        prelude::Value::{self, Str},
    },
};

// `->>` behaves exactly like `->`, except the extracted value is always returned as
// text (or NULL). Selector handling itself (string/int keys, negative indices, CAST
// selectors, ...) is already covered by `select_arrow_value` in `arrow.rs`, so these
// tests focus on the text-conversion behavior `->>` adds on top of it.
test_case!(long_arrow, {
    let g = get_tester!();

    g.run("CREATE TABLE LongArrowSample (object MAP, array LIST);")
        .await;

    g.run(
        r#"
        INSERT INTO LongArrowSample VALUES (
            '{"id":1,"name":"Han","price":4.25,"active":true,"nested":{"role":"admin"}}',
            '[1,"two",true,4.25,null]'
        );
        "#,
    )
    .await;

    g.named_test(
        "map values of every type are converted to text, missing key is NULL",
        "SELECT object->>'id' AS id, object->>'name' AS name, object->>'price' AS price, object->>'active' AS active, object->>'nested' AS nested, object->>'missing' AS missing FROM LongArrowSample;",
        Ok(select_with_null!(
            id | name | price | active | nested | missing;
            Value::Str("1".to_owned()) Value::Str("Han".to_owned()) Value::Str("4.25".to_owned()) Value::Str("TRUE".to_owned()) Value::Str(r#"{"role":"admin"}"#.to_owned()) Value::Null
        )),
    )
    .await;

    g.named_test(
        "list index/string-index/negative/out-of-range/null-element",
        "SELECT array->>0 AS by_index, array->>'1' AS by_string_index, array->>(-1) AS negative, array->>100 AS out_of_range, array->>4 AS null_element FROM LongArrowSample;",
        Ok(select_with_null!(
            by_index | by_string_index | negative | out_of_range | null_element;
            Value::Str("1".to_owned()) Value::Str("two".to_owned()) Value::Null Value::Null Value::Null
        )),
    )
    .await;

    g.named_test(
        "NULL base or NULL selector short-circuits to NULL",
        "SELECT NULL->>'k' AS base_null, object->>NULL AS selector_null FROM LongArrowSample;",
        Ok(select_with_null!(base_null | selector_null; Value::Null Value::Null)),
    )
    .await;

    g.test(
        "SELECT object->'nested'->>'role' AS result FROM LongArrowSample;",
        Ok(select!(result Str; "admin".to_owned())),
    )
    .await;

    g.test(
        "SELECT array->>CAST(1 AS INT16) AS result FROM LongArrowSample;",
        Ok(select!(result Str; "two".to_owned())),
    )
    .await;

    g.test(
        "SELECT 1 ->> 'foo' AS result;",
        Err(EvaluateError::ArrowBaseRequiresMapOrList.into()),
    )
    .await;

    g.test(
        "SELECT object->>TRUE AS result FROM LongArrowSample;",
        Err(EvaluateError::ArrowSelectorRequiresIntegerOrString("Bool(true)".to_owned()).into()),
    )
    .await;

    g.test(
        "SELECT array->>-1 AS result FROM LongArrowSample;",
        Err(TranslateError::UnsupportedBinaryOperator("->>-".to_owned()).into()),
    )
    .await;
});
