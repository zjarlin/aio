use std::{str::FromStr, time::Duration};

use anyhow::{Context, Result, ensure};
use sha2::{Digest, Sha256};
use sqlparser::{ast::Statement, dialect::PostgreSqlDialect, parser::Parser};
use sqlx::{
    PgPool,
    postgres::{PgConnectOptions, PgPoolOptions},
};

use crate::ScopedDatabase;

pub struct DatabaseProvisioner {
    pool: PgPool,
    connection: PgConnectOptions,
}

impl DatabaseProvisioner {
    pub async fn connect(url: &str) -> Result<Self> {
        let connection = PgConnectOptions::from_str(url).context("数据库地址无效")?;
        let pool = PgPoolOptions::new()
            .max_connections(2)
            .acquire_timeout(Duration::from_secs(3))
            .connect_with(connection.clone())
            .await?;
        let public_access: bool = sqlx::query_scalar(
            "SELECT EXISTS (SELECT 1 FROM pg_namespace, LATERAL aclexplode(coalesce(nspacl, acldefault('n', nspowner))) a WHERE nspname = 'public' AND grantee = 0)",
        ).fetch_one(&pool).await?;
        ensure!(
            !public_access,
            "运行库必须先撤销 public schema 的 PUBLIC 权限"
        );
        Ok(Self { pool, connection })
    }

    pub async fn create(
        &self,
        source: &str,
        tenant: &str,
        initial_schema: &str,
    ) -> Result<ScopedDatabase> {
        let schema = namespace(source, tenant);
        let role = format!("r_{schema}");
        let owner = format!("o_{schema}");
        let mut random = [0u8; 32];
        getrandom::fill(&mut random)
            .map_err(|error| anyhow::anyhow!("生成数据库凭据失败: {error}"))?;
        let password = random
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>();
        let statements = Parser::parse_sql(&PostgreSqlDialect {}, initial_schema)?;
        ensure!(
            !statements.is_empty() && statements.len() <= 64,
            "初始化迁移数量无效"
        );
        for statement in &statements {
            let Statement::CreateTable(table) = statement else {
                anyhow::bail!("初始化迁移只接受 CREATE TABLE");
            };
            ensure!(
                table.name.0.len() == 1
                    && table.query.is_none()
                    && table.like.is_none()
                    && table.clone.is_none(),
                "初始化表不能引用其他 schema 或复制已有表"
            );
            crate::database_query::validate_nodes(statement)?;
        }
        let mut tx = self.pool.begin().await?;
        sqlx::query("SET LOCAL statement_timeout = '5s'")
            .execute(&mut *tx)
            .await?;
        // 名称只由摘要生成，密码只含十六进制字符，不接受插件提供的 SQL 标识符。
        for sql in [
            format!(
                "CREATE ROLE {role} LOGIN NOSUPERUSER NOCREATEDB NOCREATEROLE NOINHERIT NOBYPASSRLS CONNECTION LIMIT 4 PASSWORD '{password}'"
            ),
            format!(
                "CREATE ROLE {owner} NOLOGIN NOSUPERUSER NOCREATEDB NOCREATEROLE NOINHERIT NOBYPASSRLS"
            ),
            format!("CREATE SCHEMA {schema} AUTHORIZATION {owner}"),
            format!("GRANT USAGE ON SCHEMA {schema} TO {role}"),
            format!("ALTER ROLE {role} SET search_path TO {schema}"),
            format!("ALTER ROLE {role} SET statement_timeout TO '3s'"),
            format!("ALTER ROLE {role} SET idle_in_transaction_session_timeout TO '5s'"),
            format!("ALTER ROLE {role} SET work_mem TO '4MB'"),
            format!("ALTER ROLE {role} SET temp_file_limit TO '16MB'"),
            format!("SET LOCAL search_path TO {schema}"),
            format!("SET LOCAL ROLE {owner}"),
        ] {
            sqlx::query(&sql).execute(&mut *tx).await?;
        }
        for statement in statements {
            sqlx::query(&statement.to_string())
                .execute(&mut *tx)
                .await
                .context("初始化插件 schema 失败")?;
        }
        for sql in [
            format!(
                "GRANT SELECT, INSERT, UPDATE, DELETE ON ALL TABLES IN SCHEMA {schema} TO {role}"
            ),
            format!("GRANT USAGE, SELECT ON ALL SEQUENCES IN SCHEMA {schema} TO {role}"),
        ] {
            sqlx::query(&sql).execute(&mut *tx).await?;
        }
        tx.commit().await?;
        ScopedDatabase::connect_options(
            self.connection.clone().username(&role).password(&password),
            &role,
            &schema,
            source,
            tenant,
        )
        .await
    }
}

fn namespace(source: &str, tenant: &str) -> String {
    let digest = Sha256::digest(format!("{}:{source}{tenant}", source.len()).as_bytes());
    format!("p_{digest:x}")[..42].to_owned()
}
