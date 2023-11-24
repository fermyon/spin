use anyhow::Context;
use spin_sdk::{
    http::{Request, Response},
    http_component,
    mysql::{self, Decode},
};

const DB_URL_ENV: &str = "DB_URL";

#[http_component]
fn process(_req: Request) -> anyhow::Result<Response> {
    let address = std::env::var(DB_URL_ENV)?;
    let conn = mysql::Connection::open(&address)?;
    test_character_types(&conn)?;
    test_numeric_types(&conn)?;
    Ok(Response::new(200, vec![]))
}

fn test_numeric_types(conn: &mysql::Connection) -> anyhow::Result<()> {
    let create_table_sql = r#"
        CREATE TEMPORARY TABLE test_numeric_types (
            rtiny TINYINT NOT NULL,
            rsmall SMALLINT NOT NULL,
            rmedium MEDIUMINT NOT NULL,
            rint INT NOT NULL,
            rbig BIGINT NOT NULL,
            rfloat FLOAT NOT NULL,
            rdouble DOUBLE NOT NULL,
            rutiny TINYINT UNSIGNED NOT NULL,
            rusmall SMALLINT UNSIGNED NOT NULL,
            rumedium MEDIUMINT UNSIGNED NOT NULL,
            ruint INT UNSIGNED NOT NULL,
            rubig BIGINT UNSIGNED NOT NULL,
            rtinyint1 TINYINT(1) NOT NULL,
            rbool BOOLEAN NOT NULL
         );
    "#;

    conn.execute(create_table_sql, &[])?;

    let insert_sql = r#"
        INSERT INTO test_numeric_types
            (rtiny, rsmall, rmedium, rint, rbig, rfloat, rdouble, rutiny, rusmall, rumedium, ruint, rubig, rtinyint1, rbool)
        VALUES
            (0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 1);
    "#;

    conn.execute(insert_sql, &[])?;

    let sql = r#"
        SELECT
            rtiny,
            rsmall,
            rmedium,
            rint,
            rbig,
            rfloat,
            rdouble,
            rutiny,
            rusmall,
            rumedium,
            ruint,
            rubig,
            rtinyint1,
            rbool
        FROM test_numeric_types;
    "#;

    let rowset = conn.query(sql, &[])?;

    for row in rowset.rows {
        i8::decode(&row[0])?;
        i16::decode(&row[1])?;
        i32::decode(&row[2])?;
        i32::decode(&row[3])?;
        i64::decode(&row[4])?;
        f32::decode(&row[5])?;
        f64::decode(&row[6])?;
        u8::decode(&row[7])?;
        u16::decode(&row[8])?;
        u32::decode(&row[9])?;
        u32::decode(&row[10])?;
        u64::decode(&row[11])?;
        bool::decode(&row[12])?;
        bool::decode(&row[13])?;
    }

    Ok(())
}

fn test_character_types(conn: &mysql::Connection) -> anyhow::Result<()> {
    let create_table_sql = r#"
        CREATE TEMPORARY TABLE test_character_types (
            rvarchar varchar(40) NOT NULL,
            rtext text NOT NULL,
            rchar char(10) NOT NULL,
            rbinary binary(10) NOT NULL,
            rvarbinary varbinary(10) NOT NULL,
            rblob BLOB NOT NULL
         );
    "#;

    conn.execute(create_table_sql, &[])
        .context("Error executing MySQL command")?;

    let insert_sql = r#"
        INSERT INTO test_character_types
            (rvarchar, rtext, rchar, rbinary, rvarbinary, rblob)
        VALUES
            ('rvarchar', 'rtext', 'rchar', 'a', 'a', 'a');
    "#;

    conn.execute(insert_sql, &[])?;

    let sql = r#"
        SELECT
            rvarchar, rtext, rchar, rbinary, rvarbinary, rblob
        FROM test_character_types;
    "#;

    let rowset = conn.query(sql, &[])?;

    for row in rowset.rows {
        String::decode(&row[0])?;
        String::decode(&row[1])?;
        String::decode(&row[2])?;
        Vec::<u8>::decode(&row[3])?;
        Vec::<u8>::decode(&row[4])?;
        Vec::<u8>::decode(&row[5])?;
    }

    Ok(())
}
