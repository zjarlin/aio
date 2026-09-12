use anyhow::{Context, Result, bail, ensure};
use futures_util::TryStreamExt;
use sqlparser::{
    ast::{Expr, Statement, TableFactor, Visit, Visitor, visit_expressions, visit_relations},
    dialect::PostgreSqlDialect,
    parser::Parser,
};
use sqlx::{Column, Postgres, Row, Transaction, TypeInfo, ValueRef, postgres::PgArguments};
use std::ops::ControlFlow;

use crate::bindings::aio::plugin::database::{Rows, Value};

const MAX_ROWS: usize = 1_000;
const MAX_RESULT_BYTES: usize = 4 * 1024 * 1024;

pub(crate) fn validate(statement: &str) -> Result<()> {
    ensure!(statement.len() <= 64 * 1024, "SQL 语句超过配额");
    let statements =
        Parser::parse_sql(&PostgreSqlDialect {}, statement).context("解析插件 SQL 失败")?;
    ensure!(statements.len() == 1, "每次调用只允许一条 SQL");
    ensure!(
        matches!(
            &statements[0],
            Statement::Query(_)
                | Statement::Insert(_)
                | Statement::Update { .. }
                | Statement::Delete(_)
        ),
        "业务请求只允许查询和数据写入，事务与结构迁移由宿主管理"
    );
    validate_nodes(&statements[0])
}

pub(crate) fn validate_nodes(statement: &impl Visit) -> Result<()> {
    struct Tables;
    impl Visitor for Tables {
        type Break = &'static str;
        fn pre_visit_table_factor(&mut self, table: &TableFactor) -> ControlFlow<Self::Break> {
            match table {
                TableFactor::Table { args: None, .. }
                | TableFactor::Derived { .. }
                | TableFactor::NestedJoin { .. } => ControlFlow::Continue(()),
                _ => ControlFlow::Break("不能使用表函数或外部数据源"),
            }
        }
    }
    if let ControlFlow::Break(message) = statement.visit(&mut Tables) {
        bail!(message);
    }
    let relations = visit_relations(statement, |relation| {
        let name = relation.to_string().to_ascii_lowercase();
        if relation.0.len() != 1
            || name.trim_matches('"').starts_with("pg_")
            || name == "information_schema"
        {
            ControlFlow::Break("不能引用其他 schema 或系统表")
        } else {
            ControlFlow::Continue(())
        }
    });
    if let ControlFlow::Break(message) = relations {
        bail!(message);
    }
    let expressions = visit_expressions(statement, |expression| {
        if let Expr::Function(function) = expression {
            let name = function.name.to_string().to_ascii_lowercase();
            if !matches!(
                name.as_str(),
                "count"
                    | "now"
                    | "sum"
                    | "min"
                    | "max"
                    | "avg"
                    | "lower"
                    | "upper"
                    | "length"
                    | "char_length"
                    | "coalesce"
                    | "nullif"
                    | "abs"
                    | "round"
                    | "concat"
                    | "substring"
                    | "trim"
                    | "jsonb_build_object"
                    | "jsonb_build_array"
                    | "jsonb_agg"
            ) {
                return ControlFlow::Break("SQL 函数未列入受限能力集合");
            }
        }
        ControlFlow::Continue(())
    });
    if let ControlFlow::Break(message) = expressions {
        bail!(message);
    }
    Ok(())
}

fn bind<'a>(
    statement: &'a str,
    parameters: Vec<Value>,
) -> Result<sqlx::query::Query<'a, Postgres, PgArguments>> {
    validate(statement)?;
    ensure!(parameters.len() <= 256, "SQL 参数超过配额");
    let mut query = sqlx::query(statement);
    for value in parameters {
        query = match value {
            Value::Null => query.bind(crate::database_parameters::InferredNull),
            Value::Boolean(value) => query.bind(value),
            Value::Integer(value) => query.bind(value),
            Value::Real(value) => query.bind(value),
            Value::Text(value) => query.bind(value),
            Value::Bytes(value) => query.bind(value),
            Value::Json(value) => query.bind(sqlx::types::Json(serde_json::from_str::<
                serde_json::Value,
            >(&value)?)),
        };
    }
    Ok(query)
}

pub(crate) async fn execute(
    transaction: &mut Transaction<'static, Postgres>,
    statement: &str,
    parameters: Vec<Value>,
) -> Result<u64> {
    Ok(bind(statement, parameters)?
        .execute(&mut **transaction)
        .await
        .context("执行插件 SQL 失败")?
        .rows_affected())
}

pub(crate) async fn query(
    transaction: &mut Transaction<'static, Postgres>,
    statement: &str,
    parameters: Vec<Value>,
) -> Result<Rows> {
    let mut stream = bind(statement, parameters)?.fetch(&mut **transaction);
    let mut output = Rows {
        columns: Vec::new(),
        values: Vec::new(),
    };
    let mut bytes = 0;
    while let Some(row) = stream.try_next().await.context("读取插件 SQL 结果失败")? {
        ensure!(output.values.len() < MAX_ROWS, "SQL 返回行数超过配额");
        if output.columns.is_empty() {
            output.columns = row
                .columns()
                .iter()
                .map(|column| column.name().to_owned())
                .collect();
        }
        let mut values = Vec::new();
        for (index, column) in row.columns().iter().enumerate() {
            let raw = row.try_get_raw(index)?;
            let value = if raw.is_null() {
                Value::Null
            } else {
                match column.type_info().name() {
                    "BOOL" => Value::Boolean(row.try_get(index)?),
                    "INT2" => Value::Integer(i64::from(row.try_get::<i16, _>(index)?)),
                    "INT4" => Value::Integer(i64::from(row.try_get::<i32, _>(index)?)),
                    "INT8" => Value::Integer(row.try_get(index)?),
                    "FLOAT4" => Value::Real(f64::from(row.try_get::<f32, _>(index)?)),
                    "FLOAT8" => Value::Real(row.try_get(index)?),
                    "TEXT" | "VARCHAR" | "BPCHAR" | "NAME" => Value::Text(row.try_get(index)?),
                    "BYTEA" => Value::Bytes(row.try_get(index)?),
                    "JSON" | "JSONB" => Value::Json(
                        row.try_get::<sqlx::types::Json<serde_json::Value>, _>(index)?
                            .0
                            .to_string(),
                    ),
                    kind => bail!("SQL 返回类型 {kind} 需要显式转换为受支持的 WIT 类型"),
                }
            };
            bytes += match &value {
                Value::Text(value) | Value::Json(value) => value.len(),
                Value::Bytes(value) => value.len(),
                _ => 16,
            };
            ensure!(bytes <= MAX_RESULT_BYTES, "SQL 结果字节数超过配额");
            values.push(value);
        }
        output.values.push(values);
    }
    Ok(output)
}

#[cfg(test)]
mod tests {
    #[test]
    fn rejects_transaction_control_and_structure_changes() {
        for sql in [
            "COMMIT",
            "ROLLBACK",
            "SET ROLE admin",
            "CREATE TABLE x(id int)",
            "SELECT 1; DELETE FROM x",
            "COPY x TO '/tmp/x'",
            "SELECT set_config('statement_timeout', '0', false)",
            "SELECT * FROM set_config('statement_timeout', '0', false)",
            "SELECT * FROM other_tenant.counter",
            "SELECT * FROM pg_authid",
        ] {
            assert!(super::validate(sql).is_err(), "{sql}");
        }
        assert!(super::validate("SELECT 'a;b', $1::text").is_ok());
        assert!(super::validate("SELECT now()").is_ok());
        assert!(super::validate("UPDATE counter SET count=count+1 RETURNING count").is_ok());
    }
}
