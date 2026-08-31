fn push_record_criteria_predicate(
    query: &mut QueryBuilder<'_, Postgres>,
    criteria: &RecordCriteria,
) {
    for filter in &criteria.all {
        query.push(" AND ");
        push_record_filter(query, filter);
    }
    if criteria.any.is_empty() {
        return;
    }
    query.push(" AND (");
    for (index, filter) in criteria.any.iter().enumerate() {
        if index > 0 {
            query.push(" OR ");
        }
        push_record_filter(query, filter);
    }
    query.push(")");
}

fn push_record_filter(query: &mut QueryBuilder<'_, Postgres>, filter: &RecordFilter) {
    push_payload_text_expression(query, &filter.field);
    match filter.operator {
        RecordFilterOperator::Equals => {
            query.push(" = ");
            query.push_bind(filter.value.clone());
        }
        RecordFilterOperator::Contains => {
            query.push(" ILIKE ");
            query.push_bind(contains_pattern(&filter.value));
            query.push(" ESCAPE E'\\\\'");
        }
    }
}

fn push_record_sort(
    query: &mut QueryBuilder<'_, Postgres>,
    sort: Option<&RecordSort>,
    field_type: Option<&str>,
) {
    let Some(sort) = sort else {
        query.push(" ORDER BY created_at_ms, id");
        return;
    };
    query.push(" ORDER BY ");
    push_payload_text_expression(query, &sort.field);
    match field_type {
        Some("int" | "datetime") => {
            query.push("::bigint");
        }
        Some("decimal") => {
            query.push("::numeric");
        }
        Some("boolean") => {
            query.push("::boolean");
        }
        _ => {}
    }
    query.push(match sort.direction {
        RecordSortDirection::Ascending => " ASC NULLS LAST, created_at_ms, id",
        RecordSortDirection::Descending => " DESC NULLS LAST, created_at_ms, id",
    });
}

fn push_payload_text_expression(query: &mut QueryBuilder<'_, Postgres>, field: &str) {
    query.push("(payload ->> '");
    query.push(field.replace('\'', "''"));
    query.push("')");
}

fn contains_pattern(value: &str) -> String {
    let escaped = value
        .replace('\\', "\\\\")
        .replace('%', "\\%")
        .replace('_', "\\_");
    format!("%{escaped}%")
}

